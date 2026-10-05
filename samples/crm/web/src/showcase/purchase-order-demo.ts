import { Injectable, computed, signal } from "@angular/core";
import { AtlasEditColumn, replaceEditedRow, validateEditRow } from "@bqatlas/ui";
export interface PurchaseLine {
  id: string;
  description: string;
  quantity: number;
  price: number;
}
export interface PurchaseOrder {
  id: string;
  supplier: string;
  reference: string;
  lines: PurchaseLine[];
}
export const purchaseColumns: AtlasEditColumn<PurchaseLine>[] = [
  { key: "id", label: "Line", readonly: true },
  { key: "description", label: "Description", required: true },
  {
    key: "quantity",
    label: "Quantity",
    type: "number",
    required: true,
    min: 1,
    max: 10000,
    step: 1,
  },
  {
    key: "price",
    label: "Unit price",
    type: "number",
    required: true,
    min: 0,
    max: 1000000,
    step: 0.01,
    format: (line) => line.price.toFixed(2),
  },
];
@Injectable()
export class PurchaseOrderDemo {
  readonly orders = signal<PurchaseOrder[]>([
    {
      id: "PO-2001",
      supplier: "Northstar Supply",
      reference: "Office refit",
      lines: [
        { id: "L-001", description: "Studio desk", quantity: 4, price: 425 },
        { id: "L-002", description: "Task chair", quantity: 8, price: 189 },
      ],
    },
    {
      id: "PO-2002",
      supplier: "Meridian Studio",
      reference: "Reception lighting",
      lines: [
        { id: "L-003", description: "Desk lamp", quantity: 6, price: 65 },
      ],
    },
    {
      id: "PO-2003",
      supplier: "Acme Components",
      reference: "Display equipment",
      lines: [
        { id: "L-004", description: "Monitor arm", quantity: 10, price: 89 },
      ],
    },
  ]);
  readonly order = signal<PurchaseOrder>(structuredClone(this.orders()[0]));
  readonly rowDraft = signal<PurchaseLine | null>(null);
  readonly saving = signal(false);
  readonly error = signal("");
  readonly message = signal("");
  readonly failNext = signal(false);
  readonly dirty = computed(
    () =>
      !!this.rowDraft() ||
      JSON.stringify(this.order()) !==
        JSON.stringify(
          this.orders().find((order) => order.id === this.order().id),
        ),
  );
  readonly total = computed(() =>
    this.order().lines.reduce(
      (total, line) => total + line.quantity * line.price,
      0,
    ),
  );
  private sequence = 4;
  setLines(lines: PurchaseLine[]) {
    if (!this.saving()) {
      this.order.update((order) => ({ ...order, lines }));
      this.error.set("");
      this.message.set("");
    }
  }
  header(key: "supplier" | "reference", value: string) {
    if (!this.saving()) {
      this.order.update((order) => ({ ...order, [key]: value }));
      this.error.set("");
    }
  }
  select(id: string) {
    if (this.saving()) return;
    const order = this.orders().find((order) => order.id === id);
    if (order) {
      this.order.set(structuredClone(order));
      this.rowDraft.set(null);
      this.error.set("");
      this.message.set("");
    }
  }
  discard() {
    this.select(this.order().id);
    this.message.set("Unsaved changes discarded.");
  }
  add(line: Omit<PurchaseLine, "id">) {
    if (this.saving() || this.rowDraft()) return;
    this.setLines([
      ...this.order().lines,
      { ...line, id: `L-${String(++this.sequence).padStart(3, "0")}` },
    ]);
  }
  remove(id: string) {
    if (this.saving() || this.rowDraft()) return;
    this.setLines(this.order().lines.filter((line) => line.id !== id));
  }
  async save() {
    if (this.saving()) throw new Error("A save is already in progress.");
    this.error.set("");
    this.message.set("");
    try {
      if (!this.order().supplier.trim() || !this.order().reference.trim())
        throw new Error("Supplier and purchase reference are required.");
      if (this.rowDraft()) {
        const errors = validateEditRow(this.rowDraft()!, purchaseColumns);
        if (Object.keys(errors).length)
          throw new Error(Object.values(errors).join(" "));
        this.setLines(
          replaceEditedRow(this.order().lines, this.rowDraft()!, "id"),
        );
        this.rowDraft.set(null);
      }
      if (!this.order().lines.length)
        throw new Error("Add at least one line before saving.");
      const snapshot = structuredClone(this.order());
      this.saving.set(true);
      await new Promise((resolve) => setTimeout(resolve, 550));
      if (this.failNext()) {
        this.failNext.set(false);
        throw new Error(
          "The sample service could not save this order. Your draft is intact. Try Save order again.",
        );
      }
      this.orders.update((orders) =>
        orders.map((order) => (order.id === snapshot.id ? snapshot : order)),
      );
      this.message.set(`${snapshot.id} saved locally.`);
    } catch (error) {
      this.error.set(error instanceof Error ? error.message : "Save failed.");
      throw error;
    } finally {
      this.saving.set(false);
    }
  }
}
