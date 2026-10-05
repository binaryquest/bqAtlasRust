import {
  ChangeDetectionStrategy,
  Component,
  computed,
  signal,
} from "@angular/core";
import { JsonPipe } from "@angular/common";
import {
  FormField,
  form,
  required,
  maxLength,
  disabled,
  readonly,
} from "@angular/forms/signals";
import {
  AtlasButton,
  AtlasField,
  AtlasLookup,
  AtlasLookupColumn,
  AtlasPanel,
  AtlasTextarea,
  AtlasToggle,
} from "@bqatlas/ui";

interface CustomerRecord {
  id: string;
  name: string;
  city: string;
  balance: number;
  active: boolean;
}

@Component({
  selector: "demo-erp-controls",
  imports: [
    JsonPipe,
    FormField,
    AtlasLookup,
    AtlasTextarea,
    AtlasToggle,
    AtlasPanel,
    AtlasField,
    AtlasButton,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="lab-section-intro">
      <div>
        <h3>Pick the record, keep the context</h3>
        <p>
          Compare customer code, name, location, and balance in one dropdown.
          Store just the customer ID.
        </p>
      </div>
    </div>
    <atlas-panel title="Customer lookup">
      <atlas-field
        controlId="erp-customer"
        label="Customer"
        [required]="true"
        [error]="
          fields.customerId().touched() && fields.customerId().invalid()
            ? 'Choose a customer.'
            : ''
        "
      >
        <atlas-lookup
          controlId="erp-customer"
          ariaLabel="Customer lookup"
          [rows]="customers"
          [columns]="columns"
          [recordKey]="customerId"
          [displayWith]="customerLabel"
          [rowDisabled]="inactive"
          [formField]="fields.customerId"
          [loading]="pending()"
          [error]="failed() ? 'Customer records could not be loaded.' : ''"
          [describedBy]="
            fields.customerId().touched() && fields.customerId().invalid()
              ? 'erp-customer-error'
              : undefined
          "
          (retry)="failed.set(false)"
        />
      </atlas-field>
      <p class="lab-note">
        Try “Dhaka”, “C-002”, or “northstar”. The archived record cannot be
        selected.
      </p>
      <div class="lab-form-actions">
        <button atlasButton (click)="pending.set(!pending())">
          {{ pending() ? "Finish loading" : "Simulate loading" }}
        </button>
        <button atlasButton (click)="failed.set(!failed())">
          {{ failed() ? "Clear error" : "Simulate error" }}
        </button>
      </div>
    </atlas-panel>
    <atlas-panel title="Signal Forms: delivery preferences">
      <atlas-field
        controlId="erp-notes"
        label="Delivery notes"
        [error]="
          fields.notes().touched() && fields.notes().invalid()
            ? 'Keep notes within 160 characters.'
            : ''
        "
      >
        <atlas-textarea
          controlId="erp-notes"
          ariaLabel="Delivery notes"
          [formField]="fields.notes"
          placeholder="Instructions for the delivery team…"
        />
      </atlas-field>
      <atlas-toggle
        controlId="erp-notify"
        label="Send delivery notifications"
        [formField]="fields.notify"
      />
      <div class="lab-form-actions">
        <button atlasButton variant="primary" (click)="validate()">
          Validate
        </button>
        <button atlasButton (click)="reset()">Reset form</button>
        <button atlasButton (click)="locked.update(invert)">
          {{ locked() ? "Enable form" : "Disable form" }}
        </button>
        <button atlasButton (click)="viewOnly.update(invert)">
          {{ viewOnly() ? "Make editable" : "Make read-only" }}
        </button>
      </div>
      @if (message()) {
        <p role="status" class="lab-note">{{ message() }}</p>
      }
    </atlas-panel>
    <atlas-panel title="Live signal model">
      <pre class="lab-value-preview">{{ data() | json }}</pre>
      <div class="lab-state-row">
        <span>{{
          fields().disabled()
            ? "Disabled"
            : viewOnly()
              ? "Read-only"
              : fields().valid()
                ? "Valid"
                : "Invalid"
        }}</span
        ><span
          >{{ fields().dirty() ? "Dirty" : "Pristine" }} ·
          {{ fields().touched() ? "Touched" : "Untouched" }}</span
        >
      </div>
      <p class="lab-note">
        Selected record: {{ selected()?.name || "None" }}. Changing the search
        never changes this model until you select a record.
      </p>
    </atlas-panel>
  `,
})
export class ErpControlExamples {
  readonly initial = {
    customerId: null as string | null,
    notes: "",
    notify: true,
  };
  readonly data = signal({ ...this.initial });
  readonly locked = signal(false);
  readonly viewOnly = signal(false);
  readonly pending = signal(false);
  readonly failed = signal(false);
  readonly message = signal("");
  readonly fields = form(this.data, (schema) => {
    required(schema.customerId, { message: "Choose a customer." });
    maxLength(schema.notes, 160);
    disabled(schema, { when: () => this.locked() });
    readonly(schema, { when: () => this.viewOnly() });
  });
  readonly customers: CustomerRecord[] = [
    {
      id: "C-001",
      name: "Acme Studio",
      city: "Dhaka",
      balance: 12500,
      active: true,
    },
    {
      id: "C-002",
      name: "Northstar Supply",
      city: "London",
      balance: 3250,
      active: true,
    },
    {
      id: "C-003",
      name: "Forma & Co.",
      city: "Berlin",
      balance: 0,
      active: true,
    },
    {
      id: "C-004",
      name: "Meridian Labs",
      city: "Dhaka",
      balance: 840,
      active: true,
    },
    {
      id: "C-005",
      name: "Oak & Stone",
      city: "Toronto",
      balance: 1920,
      active: true,
    },
    {
      id: "C-006",
      name: "Legacy Trading (archived)",
      city: "Dhaka",
      balance: 780,
      active: false,
    },
    {
      id: "C-007",
      name: "Linear Works",
      city: "Tokyo",
      balance: 2500,
      active: true,
    },
  ];
  readonly customerId = (row: CustomerRecord) => row.id;
  readonly customerLabel = (row: CustomerRecord) => row.name;
  readonly inactive = (row: CustomerRecord) => !row.active;
  readonly invert = (value: boolean) => !value;
  readonly columns: AtlasLookupColumn<CustomerRecord>[] = [
    { key: "id", label: "Code", width: 85 },
    { key: "name", label: "Customer" },
    { key: "city", label: "City", width: 100 },
    {
      key: "balance",
      label: "Balance (USD)",
      width: 120,
      align: "right",
      format: (row) =>
        row.balance.toLocaleString("en-US", { minimumFractionDigits: 2 }),
    },
  ];
  readonly selected = computed(() =>
    this.customers.find((c) => c.id === this.data().customerId),
  );
  validate() {
    this.fields().markAsTouched();
    this.message.set(
      this.fields().disabled()
        ? "Enable the form to validate."
        : this.viewOnly()
          ? "This form is read-only."
          : this.fields().valid()
            ? "Form is valid. Ready for your API."
            : "Choose a customer to continue.",
    );
  }
  reset() {
    this.fields().reset({ ...this.initial });
    this.message.set("");
  }
}
