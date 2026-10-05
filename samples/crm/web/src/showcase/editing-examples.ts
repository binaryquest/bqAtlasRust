import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  signal,
} from "@angular/core";
import {
  AtlasButton,
  AtlasCommand,
  AtlasCommandToolbar,
  AtlasDialog,
  AtlasEditableGrid,
  AtlasInput,
  AtlasMasterDetail,
  validateEditRow,
} from "@bqatlas/ui";
import {
  PurchaseLine,
  PurchaseOrderDemo,
  purchaseColumns,
} from "./purchase-order-demo";
@Component({
  selector: "demo-editing",
  imports: [
    AtlasButton,
    AtlasCommandToolbar,
    AtlasDialog,
    AtlasEditableGrid,
    AtlasInput,
    AtlasMasterDetail,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="lab-section-intro">
      <div>
        <h3>Purchase orders</h3>
        <p>
          Edit line items, save a draft, and move between records without losing
          your work.
        </p>
      </div>
    </div>
    <atlas-master-detail
      [(detailOpen)]="detailOpen"
      masterLabel="Purchase orders"
      detailLabel="Purchase order editor"
    >
      <div atlasMaster class="purchase-master">
        <h3>Orders</h3>
        @for (order of store.orders(); track order.id) {
          <button
            type="button"
            class="purchase-record"
            [class.active]="store.order().id === order.id"
            [attr.aria-current]="store.order().id === order.id ? 'true' : null"
            [disabled]="store.saving()"
            (click)="choose(order.id)"
          >
            <strong>{{ order.id }}</strong
            ><span>{{ order.supplier }}</span
            ><small>{{ order.reference }}</small>
          </button>
        }
      </div>
      <div atlasDetail>
        <atlas-command-toolbar
          label="Purchase order actions"
          [commands]="commands()"
          [disabled]="store.saving()"
          (command)="command($event)"
          >{{
            store.saving()
              ? "Saving…"
              : store.dirty()
                ? "Unsaved draft"
                : "Saved"
          }}</atlas-command-toolbar
        >
        <div class="purchase-heading">
          <h3>{{ store.order().id }}</h3>
          <span>USD · Dummy data</span>
        </div>
        <div class="form-grid purchase-fields">
          <label
            >Supplier<select
              atlasInput
              aria-label="Purchase supplier"
              [value]="store.order().supplier"
              [disabled]="store.saving()"
              (change)="store.header('supplier', $any($event.target).value)"
            >
              <option>Northstar Supply</option>
              <option>Meridian Studio</option>
              <option>Acme Components</option>
            </select></label
          ><label
            >Purchase reference<input
              atlasInput
              aria-label="Purchase reference"
              [value]="store.order().reference"
              [disabled]="store.saving()"
              (input)="store.header('reference', $any($event.target).value)"
          /></label>
        </div>
        @if (store.error()) {
          <p class="atlas-edit-alert" role="alert">{{ store.error() }}</p>
        }
        @if (store.message()) {
          <p role="status" class="atlas-edit-status">{{ store.message() }}</p>
        }
        <atlas-editable-grid
          label="Purchase order lines"
          [rows]="store.order().lines"
          (rowsChange)="store.setLines($event)"
          [(draft)]="store.rowDraft"
          [columns]="columns"
          keyField="id"
          [disabled]="store.saving()"
          [removable]="true"
          (removeRequested)="removeLine.set($event); showDialog('remove')"
        />
        <div class="purchase-total">
          {{ store.order().lines.length }} lines
          <strong>Total {{ money(store.total()) }}</strong>
        </div>
        <label class="purchase-failure"
          ><input
            type="checkbox"
            [checked]="store.failNext()"
            [disabled]="store.saving()"
            (change)="store.failNext.set($any($event.target).checked)"
          />Fail next save</label
        >
        <p class="atlas-muted">
          Tab moves between fields. Ctrl/Cmd+Enter applies a row; Escape cancels
          it. Save order commits the order draft.
        </p>
      </div>
    </atlas-master-detail>
    <atlas-dialog
      [title]="dialogTitle()"
      [(open)]="dialogOpen"
      (dismissed)="pendingId.set('')"
    >
      @switch (dialogKind()) {
        @case ("add") {
          <div class="form-grid purchase-fields">
            <label
              >Description<input
                atlasInput
                aria-label="New line description"
                [value]="newLine().description"
                (input)="
                  updateNew('description', $any($event.target).value)
                " /></label
            ><label
              >Quantity<input
                atlasInput
                type="number"
                aria-label="New line quantity"
                min="1"
                step="1"
                [value]="newLine().quantity"
                (input)="
                  updateNew('quantity', $any($event.target).value)
                " /></label
            ><label
              >Unit price<input
                atlasInput
                type="number"
                aria-label="New line unit price"
                min="0"
                step="0.01"
                [value]="newLine().price"
                (input)="updateNew('price', $any($event.target).value)"
            /></label>
          </div>
          @if (addError()) {
            <p role="alert" class="atlas-edit-alert">{{ addError() }}</p>
          }
        }
        @case ("remove") {
          <p>
            Remove {{ removeLine()?.description }} from this order draft? Save
            order will commit the removal.
          </p>
        }
        @default {
          <p>
            This order has unsaved changes. Discarding restores the last saved
            version, including any row being edited.
          </p>
        }
      }
      <div atlasDialogActions class="purchase-dialog-actions">
        <button
          atlasButton
          type="button"
          (click)="dialogOpen.set(false); pendingId.set('')"
        >
          {{ dialogKind() === "discard" ? "Keep editing" : "Cancel" }}</button
        ><button
          atlasButton
          type="button"
          [variant]="dialogKind() === 'add' ? 'primary' : 'danger'"
          (click)="confirm()"
        >
          {{
            dialogKind() === "add"
              ? "Add line"
              : dialogKind() === "remove"
                ? "Remove line"
                : "Discard changes"
          }}
        </button>
      </div>
    </atlas-dialog>
  `,
})
export class EditingExamples {
  readonly store = inject(PurchaseOrderDemo);
  readonly columns = purchaseColumns;
  readonly detailOpen = signal(false);
  readonly dialogOpen = signal(false);
  readonly dialogKind = signal<"add" | "discard" | "remove">("add");
  readonly pendingId = signal("");
  readonly removeLine = signal<PurchaseLine | null>(null);
  readonly newLine = signal({
    id: "new",
    description: "",
    quantity: 1,
    price: 0,
  });
  readonly addError = signal("");
  readonly dialogTitle = computed(() =>
    this.dialogKind() === "add"
      ? "Add order line"
      : this.dialogKind() === "remove"
        ? "Remove order line"
        : "Unsaved order changes",
  );
  readonly commands = computed<AtlasCommand[]>(() => [
    {
      id: "save",
      label: "Save order",
      icon: "save",
      primary: true,
      disabled: !this.store.dirty(),
      group: "save",
    },
    {
      id: "discard",
      label: "Discard changes",
      disabled: !this.store.dirty(),
      group: "save",
    },
    {
      id: "add",
      label: "Add line",
      icon: "plus",
      disabled: !!this.store.rowDraft(),
      group: "lines",
    },
  ]);
  money(value: number) {
    return new Intl.NumberFormat("en-US", {
      style: "currency",
      currency: "USD",
    }).format(value);
  }
  command(id: string) {
    if (id === "save") void this.store.save().catch(() => {});
    else if (id === "add") {
      this.newLine.set({ id: "new", description: "", quantity: 1, price: 0 });
      this.addError.set("");
      this.showDialog("add");
    } else if (id === "discard") {
      this.pendingId.set("");
      this.showDialog("discard");
    }
  }
  showDialog(kind: "add" | "discard" | "remove") {
    this.dialogKind.set(kind);
    this.dialogOpen.set(true);
  }
  choose(id: string) {
    if (id === this.store.order().id) {
      this.detailOpen.set(true);
      return;
    }
    if (this.store.dirty()) {
      this.pendingId.set(id);
      this.showDialog("discard");
    } else {
      this.store.select(id);
      this.detailOpen.set(true);
    }
  }
  updateNew(key: "description" | "quantity" | "price", value: string) {
    this.newLine.update((row) => ({
      ...row,
      [key]: key === "description" ? value : value === "" ? NaN : Number(value),
    }));
  }
  confirm() {
    if (this.dialogKind() === "add") {
      const errors = validateEditRow(this.newLine(), this.columns);
      if (Object.keys(errors).length) {
        this.addError.set(Object.values(errors).join(" "));
        return;
      }
      this.store.add(this.newLine());
    } else if (this.dialogKind() === "remove")
      this.store.remove(this.removeLine()!.id);
    else {
      const id = this.pendingId();
      this.store.discard();
      if (id) {
        this.store.select(id);
        this.detailOpen.set(true);
      }
      this.pendingId.set("");
    }
    this.dialogOpen.set(false);
  }
}
