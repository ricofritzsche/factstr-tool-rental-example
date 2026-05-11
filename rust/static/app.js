const registerForm = document.querySelector("#register-form");
const registerButton = document.querySelector("#register-button");
const randomToolButton = document.querySelector("#random-tool-button");
const refreshButton = document.querySelector("#refresh-button");
const inventoryList = document.querySelector("#inventory-list");
const inventoryStatus = document.querySelector("#inventory-status");
const messageBox = document.querySelector("#message");
const liveStatus = document.querySelector("#live-status");
const liveStatusText = document.querySelector("#live-status-text");
const itemTemplate = document.querySelector("#inventory-item-template");
const checkoutDialog = document.querySelector("#checkout-dialog");
const checkoutForm = document.querySelector("#checkout-form");
const checkoutSubmitButton = document.querySelector("#checkout-submit-button");

let inventoryRequestInFlight = false;
let checkoutRequestInFlight = false;

const errorMessages = {
  invalid_request: "The request payload was not accepted.",
  empty_serial_number: "Serial number is required.",
  empty_name: "Tool name is required.",
  empty_category: "Category is required.",
  serial_number_already_registered: "That serial number is already registered.",
  empty_checked_out_to: "Checked out to is required.",
  missing_checked_out_at: "Checkout time is required.",
  missing_due_back_at: "Due back time is required.",
  due_back_must_be_later_than_checked_out: "Due back time must be later than checkout time.",
  tool_not_registered: "The selected tool is not registered.",
  tool_already_checked_out: "The selected tool is already checked out.",
  missing_returned_at: "Return time is required.",
  tool_not_checked_out: "The selected tool is not currently checked out.",
  store_error: "The server could not complete the request."
};

const sampleToolCatalog = [
  {
    name: "Rotary Hammer",
    category: "drilling",
    manufacturer: "Bosch",
    model: "GBH 2-26",
    home_location: "warehouse-a",
    initial_condition: "usable"
  },
  {
    name: "Circular Saw",
    category: "saws",
    manufacturer: "Makita",
    model: "HS7601",
    home_location: "warehouse-b",
    initial_condition: "usable"
  },
  {
    name: "Angle Grinder",
    category: "grinding",
    manufacturer: "DeWalt",
    model: "DWE4257",
    home_location: "warehouse-a",
    initial_condition: "usable"
  },
  {
    name: "Impact Driver",
    category: "fastening",
    manufacturer: "Milwaukee",
    model: "M18 FID3",
    home_location: "warehouse-c",
    initial_condition: "usable"
  },
  {
    name: "Orbital Sander",
    category: "finishing",
    manufacturer: "Festool",
    model: "ETS EC 150",
    home_location: "warehouse-b",
    initial_condition: "usable"
  }
];

const sampleCheckedOutToValues = [
  "Team Alpha",
  "Team Bravo",
  "Site Crew 7",
  "Workshop Bench",
  "Maintenance Van 2",
  "Project Falcon",
  "Install Team North"
];

function setMessage(type, text) {
  if (!text) {
    messageBox.hidden = true;
    messageBox.textContent = "";
    messageBox.dataset.type = "";
    return;
  }

  messageBox.hidden = false;
  messageBox.dataset.type = type;
  messageBox.textContent = text;
}

function humanizeErrorCode(code) {
  return errorMessages[code] ?? "The request could not be completed.";
}

function setLiveStatus(state, text) {
  liveStatus.dataset.state = state;
  liveStatusText.textContent = text;
}

function optionalFieldValue(formData, fieldName) {
  const value = formData.get(fieldName)?.toString().trim() ?? "";
  return value === "" ? undefined : value;
}

function randomItem(items) {
  return items[Math.floor(Math.random() * items.length)];
}

function randomSerialNumber() {
  const letters = Math.random().toString(36).slice(2, 5).toUpperCase();
  const digits = Math.floor(1000 + Math.random() * 9000);
  return `SN-${letters}-${digits}`;
}

function randomCheckedOutToValue() {
  return randomItem(sampleCheckedOutToValues);
}

function fillRandomTool() {
  const sample = randomItem(sampleToolCatalog);

  registerForm.elements.serial_number.value = randomSerialNumber();
  registerForm.elements.name.value = sample.name;
  registerForm.elements.category.value = sample.category;
  registerForm.elements.manufacturer.value = sample.manufacturer;
  registerForm.elements.model.value = sample.model;
  registerForm.elements.home_location.value = sample.home_location;
  registerForm.elements.initial_condition.value = sample.initial_condition;

  setMessage("success", "Random tool details generated.");
}

function formatDateTime(value) {
  if (!value) {
    return "—";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString();
}

function defaultUseLocation(item) {
  return item.current_location || item.home_location || "unassigned";
}

function randomDueBackAtInputValue() {
  const daysAhead = Math.floor(Math.random() * 10) + 1;
  const dueBackAt = new Date();
  dueBackAt.setDate(dueBackAt.getDate() + daysAhead);

  const year = dueBackAt.getFullYear();
  const month = String(dueBackAt.getMonth() + 1).padStart(2, "0");
  const day = String(dueBackAt.getDate()).padStart(2, "0");
  const hours = String(dueBackAt.getHours()).padStart(2, "0");
  const minutes = String(dueBackAt.getMinutes()).padStart(2, "0");

  return `${year}-${month}-${day}T${hours}:${minutes}`;
}

function showLoadingState(isLoading) {
  inventoryRequestInFlight = isLoading;
  refreshButton.disabled = isLoading;
  inventoryStatus.textContent = isLoading ? "Loading inventory…" : "";
}

function setActionButtonsDisabled(isDisabled) {
  for (const button of document.querySelectorAll("[data-item-action]")) {
    button.disabled = isDisabled;
  }
}

function itemDetail(label, value) {
  const wrapper = document.createElement("div");
  const term = document.createElement("dt");
  const definition = document.createElement("dd");
  term.textContent = label;
  definition.textContent = value || "—";
  wrapper.append(term, definition);
  return wrapper;
}

function renderInventory(items) {
  inventoryList.innerHTML = "";

  if (items.length === 0) {
    inventoryStatus.textContent = "No tools registered yet.";
    return;
  }

  inventoryStatus.textContent = "";

  // TODO: If the UI should show newest registrations first, the backend projection
  // needs to expose a stable registration ordering field such as a sequence number.
  // For now we intentionally render the inventory in the exact order returned by GET /tools.
  for (const item of items) {
    const fragment = itemTemplate.content.cloneNode(true);
    const root = fragment.querySelector(".inventory-item");
    const toolName = fragment.querySelector(".tool-name");
    const toolMeta = fragment.querySelector(".tool-meta");
    const statusBadge = fragment.querySelector(".status-badge");
    const details = fragment.querySelector(".tool-details");
    const actions = fragment.querySelector(".item-actions");

    toolName.textContent = item.name;
    toolMeta.textContent = [item.serial_number, item.category].filter(Boolean).join(" • ");
    statusBadge.textContent = item.status === "checked_out" ? "Checked Out" : "Available";
    statusBadge.dataset.status = item.status;

    details.append(
      itemDetail("Manufacturer", item.manufacturer),
      itemDetail("Model", item.model),
      itemDetail("Current Location", item.current_location),
      itemDetail("Status", item.status),
      itemDetail("Checked Out To", item.status === "checked_out" ? item.checked_out_to : null),
      itemDetail("Due Back At", item.status === "checked_out" ? formatDateTime(item.due_back_at) : null)
    );

    if (item.status === "available") {
      const checkoutButton = document.createElement("button");
      checkoutButton.type = "button";
      checkoutButton.dataset.itemAction = "checkout";
      checkoutButton.textContent = "Checkout";
      checkoutButton.addEventListener("click", () => openCheckoutDialog(item));
      actions.append(checkoutButton);
    } else {
      const returnButton = document.createElement("button");
      returnButton.type = "button";
      returnButton.dataset.itemAction = "return";
      returnButton.textContent = "Return";
      returnButton.addEventListener("click", () => returnTool(item));
      actions.append(returnButton);
    }

    inventoryList.append(root);
  }
}

async function readApiError(response) {
  let payload = null;
  try {
    payload = await response.json();
  } catch {
    payload = null;
  }

  const code = payload?.code;
  throw new Error(code ? humanizeErrorCode(code) : "The request could not be completed.");
}

async function refreshInventory() {
  if (inventoryRequestInFlight) {
    return;
  }

  showLoadingState(true);

  try {
    const response = await fetch("/tools");
    if (!response.ok) {
      await readApiError(response);
    }

    const payload = await response.json();
    renderInventory(payload.items ?? []);
  } catch (error) {
    setMessage("error", error.message);
    inventoryStatus.textContent = "Unable to load inventory.";
  } finally {
    showLoadingState(false);
  }
}

async function registerTool(event) {
  event.preventDefault();
  setMessage(null, "");
  registerButton.disabled = true;

  try {
    const formData = new FormData(registerForm);
    const response = await fetch("/tools", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        serial_number: formData.get("serial_number")?.toString().trim() ?? "",
        name: formData.get("name")?.toString().trim() ?? "",
        category: formData.get("category")?.toString().trim() ?? "",
        manufacturer: optionalFieldValue(formData, "manufacturer"),
        model: optionalFieldValue(formData, "model"),
        home_location: optionalFieldValue(formData, "home_location"),
        initial_condition: optionalFieldValue(formData, "initial_condition")
      })
    });

    if (!response.ok) {
      await readApiError(response);
    }

    registerForm.reset();
    setMessage("success", "Tool registered.");
    await refreshInventory();
  } catch (error) {
    setMessage("error", error.message);
  } finally {
    registerButton.disabled = false;
  }
}

function openCheckoutDialog(item) {
  setMessage(null, "");
  checkoutForm.reset();
  checkoutForm.elements.tool_id.value = item.tool_id;
  checkoutForm.elements.checked_out_to.value = randomCheckedOutToValue();
  checkoutForm.elements.due_back_at.value = randomDueBackAtInputValue();
  checkoutDialog.showModal();
}

async function checkoutTool(event) {
  event.preventDefault();
  if (checkoutRequestInFlight) {
    return;
  }

  checkoutRequestInFlight = true;
  checkoutSubmitButton.disabled = true;
  setActionButtonsDisabled(true);
  setMessage(null, "");

  try {
    const formData = new FormData(checkoutForm);
    const toolId = formData.get("tool_id")?.toString() ?? "";
    const now = new Date().toISOString();
    const dueBackAtInput = formData.get("due_back_at")?.toString() ?? "";
    const dueBackAt = new Date(dueBackAtInput).toISOString();

    const response = await fetch(`/tools/${toolId}/checkout`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        checked_out_to: formData.get("checked_out_to")?.toString().trim() ?? "",
        checked_out_at: now,
        due_back_at: dueBackAt,
        use_location: defaultUseLocation(findRenderedItem(toolId)),
        condition_at_checkout: "usable"
      })
    });

    if (!response.ok) {
      await readApiError(response);
    }

    checkoutDialog.close();
    setMessage("success", "Tool checked out.");
    await refreshInventory();
  } catch (error) {
    setMessage("error", error.message);
  } finally {
    checkoutRequestInFlight = false;
    checkoutSubmitButton.disabled = false;
    setActionButtonsDisabled(false);
  }
}

function findRenderedItem(toolId) {
  return currentInventoryItems.find((item) => item.tool_id === toolId) ?? {};
}

let currentInventoryItems = [];

const originalRenderInventory = renderInventory;
renderInventory = function renderAndTrack(items) {
  currentInventoryItems = items;
  originalRenderInventory(items);
};

async function returnTool(item) {
  setMessage(null, "");
  setActionButtonsDisabled(true);

  try {
    const response = await fetch(`/tools/${item.tool_id}/return`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        returned_at: new Date().toISOString(),
        returned_to_location: item.home_location || item.current_location || "unassigned",
        condition_at_return: "usable"
      })
    });

    if (!response.ok) {
      await readApiError(response);
    }

    setMessage("success", "Tool returned.");
    await refreshInventory();
  } catch (error) {
    setMessage("error", error.message);
  } finally {
    setActionButtonsDisabled(false);
  }
}

function connectInventoryEvents() {
  const eventSource = new EventSource("/tools/events");

  eventSource.addEventListener("open", () => {
    setLiveStatus("connected", "Live updates connected");
  });

  eventSource.addEventListener("inventory-changed", async () => {
    await refreshInventory();
  });

  eventSource.addEventListener("error", () => {
    setLiveStatus("disconnected", "Live updates disconnected. Manual refresh still works.");
  });
}

registerForm.addEventListener("submit", registerTool);
randomToolButton.addEventListener("click", fillRandomTool);
refreshButton.addEventListener("click", refreshInventory);
checkoutForm.addEventListener("submit", checkoutTool);
document
  .querySelector("[data-close-dialog]")
  .addEventListener("click", () => checkoutDialog.close());

connectInventoryEvents();
refreshInventory();
