use tokio::sync::broadcast;

const INVENTORY_CHANGE_CHANNEL_CAPACITY: usize = 64;

#[derive(Clone)]
pub struct InventoryChangeNotifier {
    sender: broadcast::Sender<()>,
}

impl InventoryChangeNotifier {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(INVENTORY_CHANGE_CHANNEL_CAPACITY);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.sender.subscribe()
    }

    pub fn notify_inventory_changed(&self) {
        let _ = self.sender.send(());
    }
}

impl Default for InventoryChangeNotifier {
    fn default() -> Self {
        Self::new()
    }
}
