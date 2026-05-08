use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

use factstr::{
    DurableStream, EventFilter, EventQuery, EventStore, EventStream, HandleStream,
    StreamHandlerError,
};
use tokio::runtime::{Handle, RuntimeFlavor};

use crate::events::{
    TOOL_CHECKED_OUT_EVENT_TYPE, TOOL_REGISTERED_EVENT_TYPE, TOOL_RETURNED_EVENT_TYPE,
};
use crate::features::get_inventory::apply_fact::{InventoryFact, apply_fact, decode_fact};
use crate::features::get_inventory::inventory_item::InventoryItem;
use crate::features::get_inventory::inventory_projection::{
    InventoryProjectionError, InventoryProjectionState,
};
use crate::features::get_inventory::projection_schema::schema_statements;
use crate::features::get_inventory::projection_store::ProjectionStore;
use crate::projection_database::ProjectionDatabase;

const GET_INVENTORY_DURABLE_STREAM_NAME: &str = "get_inventory";

#[derive(Clone)]
pub struct InventoryProjection {
    state: Arc<Mutex<InventoryProjectionState>>,
    stream: Option<Arc<EventStream>>,
}

impl InventoryProjection {
    pub fn empty() -> Self {
        Self::from_items(Vec::new())
    }

    pub fn from_items(items: Vec<InventoryItem>) -> Self {
        Self {
            state: Arc::new(Mutex::new(InventoryProjectionState::from_items(items))),
            stream: None,
        }
    }

    pub fn snapshot(&self) -> Result<Vec<InventoryItem>, InventoryProjectionError> {
        Ok(self.lock_state()?.list_items())
    }

    pub fn is_live(&self) -> bool {
        self.stream.is_some()
    }

    pub(crate) fn state_handle(&self) -> Arc<Mutex<InventoryProjectionState>> {
        Arc::clone(&self.state)
    }

    pub(crate) fn with_stream(self, stream: EventStream) -> Self {
        Self {
            state: self.state,
            stream: Some(Arc::new(stream)),
        }
    }

    fn lock_state(
        &self,
    ) -> Result<MutexGuard<'_, InventoryProjectionState>, InventoryProjectionError> {
        self.state
            .lock()
            .map_err(|_| InventoryProjectionError::LockPoisoned)
    }
}

pub async fn start_projection(
    store: &impl EventStore,
    projection_database: ProjectionDatabase,
) -> Result<InventoryProjection, InventoryProjectionError> {
    projection_database
        .initialize_schema(GET_INVENTORY_DURABLE_STREAM_NAME, &schema_statements())
        .await
        .map_err(InventoryProjectionError::store_error)?;

    let projection_store = ProjectionStore::open(&projection_database).await?;
    let items = projection_store.list_items().await?;
    let projection = InventoryProjection::from_items(items);

    attach_durable_stream(store, projection, Some(projection_store))
}

pub fn start_projection_in_memory(
    store: &impl EventStore,
) -> Result<InventoryProjection, InventoryProjectionError> {
    attach_durable_stream(store, InventoryProjection::empty(), None)
}

fn attach_durable_stream(
    store: &impl EventStore,
    projection: InventoryProjection,
    projection_store: Option<ProjectionStore>,
) -> Result<InventoryProjection, InventoryProjectionError> {
    let projection_state = projection.state_handle();

    let event_query = EventQuery::all().with_filters([EventFilter::for_event_types([
        TOOL_REGISTERED_EVENT_TYPE,
        TOOL_CHECKED_OUT_EVENT_TYPE,
        TOOL_RETURNED_EVENT_TYPE,
    ])]);
    let durable_stream = DurableStream::new(GET_INVENTORY_DURABLE_STREAM_NAME);
    let handle: HandleStream = Arc::new(move |event_records| {
        let mut projection_state = projection_state
            .lock()
            .map_err(|_| StreamHandlerError::new("inventory projection lock poisoned"))?;

        for event_record in event_records {
            let fact = decode_fact(&event_record)
                .map_err(|error| StreamHandlerError::new(error.to_string()))?;

            if let Some(projection_store) = &projection_store {
                persist_fact(projection_store, &fact)
                    .map_err(|error| StreamHandlerError::new(error.to_string()))?;
            }

            apply_fact(&mut projection_state, &fact);
        }

        Ok(())
    });

    let event_stream = store
        .stream_to_durable(&durable_stream, &event_query, handle)
        .map_err(InventoryProjectionError::store_error)?;

    Ok(projection.with_stream(event_stream))
}

fn persist_fact(
    projection_store: &ProjectionStore,
    fact: &InventoryFact,
) -> Result<(), InventoryProjectionError> {
    if matches!(fact, InventoryFact::Ignored) {
        return Ok(());
    }

    let projection_store = projection_store.clone();
    let fact = fact.clone();

    match Handle::try_current() {
        Ok(handle) => match handle.runtime_flavor() {
            RuntimeFlavor::MultiThread => tokio::task::block_in_place(move || {
                handle.block_on(persist_fact_async(&projection_store, &fact))
            }),
            RuntimeFlavor::CurrentThread => {
                persist_fact_on_temporary_thread(projection_store, fact)
            }
            _ => persist_fact_on_local_runtime(projection_store, fact),
        },
        Err(_) => persist_fact_on_local_runtime(projection_store, fact),
    }
}

fn persist_fact_on_temporary_thread(
    projection_store: ProjectionStore,
    fact: InventoryFact,
) -> Result<(), InventoryProjectionError> {
    thread::spawn(move || persist_fact_on_local_runtime(projection_store, fact))
        .join()
        .map_err(|_| {
            InventoryProjectionError::store_error("projection persistence thread panicked")
        })?
}

fn persist_fact_on_local_runtime(
    projection_store: ProjectionStore,
    fact: InventoryFact,
) -> Result<(), InventoryProjectionError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(InventoryProjectionError::store_error)?;

    runtime.block_on(async move { persist_fact_async(&projection_store, &fact).await })
}

async fn persist_fact_async(
    projection_store: &ProjectionStore,
    fact: &InventoryFact,
) -> Result<(), InventoryProjectionError> {
    match fact {
        InventoryFact::Registered(payload) => projection_store.apply_registered(payload).await,
        InventoryFact::CheckedOut(payload) => projection_store.apply_checked_out(payload).await,
        InventoryFact::Returned(payload) => projection_store.apply_returned(payload).await,
        InventoryFact::Ignored => Ok(()),
    }
}
