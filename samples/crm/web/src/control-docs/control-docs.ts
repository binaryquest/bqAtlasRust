import {
  Component,
  ElementRef,
  viewChild,
  ChangeDetectionStrategy,
  signal,
  computed,
  inject,
} from "@angular/core";
import { FormsModule } from "@angular/forms";
import {
  AtlasButton,
  AtlasInput,
  AtlasTextarea,
  AtlasSelect,
  AtlasLookup,
  AtlasMultiSelect,
  AtlasRadioGroup,
  AtlasCheckboxGroup,
  AtlasToggle,
  AtlasDateInput,
  AtlasTabs,
  AtlasTab,
  AtlasAccordionSection,
  AtlasSplitPane,
  AtlasTable,
  AtlasDialog,
  WorkspaceService,
} from "@bqatlas/ui";
import {
  AtlasDecimalTextInput,
  ReferenceLookup,
  RestLookupProvider,
  AtlasApi,
  AtlasSession,
} from "@bqatlas/angular";
import { controlDocs } from "./catalog";
@Component({
  selector: "app-control-docs",
  imports: [
    FormsModule,
    AtlasButton,
    AtlasInput,
    AtlasTextarea,
    AtlasSelect,
    AtlasLookup,
    AtlasMultiSelect,
    AtlasRadioGroup,
    AtlasCheckboxGroup,
    AtlasToggle,
    AtlasDateInput,
    AtlasTabs,
    AtlasTab,
    AtlasAccordionSection,
    AtlasSplitPane,
    AtlasTable,
    AtlasDialog,
    AtlasDecimalTextInput,
    ReferenceLookup,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
  styleUrl: "control-docs.css",
  template: ` <section class="control-docs">
    <header class="docs-heading">
      <div>
        <p>DEVELOPER REFERENCE</p>
        <h2>Build with Atlas</h2>
        <span>One control at a time. Live examples, API notes and usage.</span>
      </div>
      <button atlasButton (click)="showcase()">Full form examples ↗</button>
    </header>
    <label class="docs-mobile-picker"
      >Choose a control
      <select
        atlasInput
        aria-label="Choose a control"
        [ngModel]="id()"
        (ngModelChange)="select($event)"
      >
        @for (item of allDocs; track item.id) {
          <option [value]="item.id">{{ item.title }}</option>
        }
      </select>
    </label>
    <div class="docs-layout">
      <nav class="docs-nav" aria-label="Control reference">
        <input
          atlasInput
          type="search"
          aria-label="Find a control"
          placeholder="Find a control…"
          [ngModel]="search()"
          (ngModelChange)="search.set($event)"
        />
        @for (group of groups; track group) {
          @if (inGroup(group).length) {
            <h3>{{ group }}</h3>
            @for (item of inGroup(group); track item.id) {
              <button
                type="button"
                [class.selected]="id() === item.id"
                [attr.aria-current]="id() === item.id ? 'page' : null"
                (click)="select(item.id)"
              >
                {{ item.title }}
              </button>
            }
          }
        }
        @if (!filtered().length) {
          <p>No matching controls.</p>
        }
      </nav>
      <article #article class="docs-article">
        <p class="docs-breadcrumb">{{ doc().group }} / {{ doc().title }}</p>
        <h1>{{ doc().title }}</h1>
        <p class="docs-description">{{ doc().description }}</p>
        <section class="docs-preview" aria-label="Live example">
          <div class="docs-section-label">LIVE EXAMPLE <span>Try it</span></div>
          <div class="docs-demo">
            @switch (id()) {
              @case ("button") {
                <button
                  atlasButton
                  variant="primary"
                  (click)="count.set(count() + 1)"
                >
                  Save example</button
                ><button atlasButton>Secondary</button
                ><button atlasButton variant="danger">Danger</button>
                <p aria-live="polite">
                  Saved {{ count() }} times in this preview.
                </p>
              }
              @case ("input") {
                <label for="docs-name">Customer name</label
                ><input atlasInput id="docs-name" [(ngModel)]="name" />
              }
              @case ("textarea") {
                <atlas-textarea
                  controlId="docs-notes"
                  ariaLabel="Notes"
                  [(value)]="notes"
                  [maxLength]="2000"
                />
              }
              @case ("select") {
                <atlas-select
                  controlId="docs-currency"
                  ariaLabel="Currency"
                  [options]="currencies"
                  [(ngModel)]="currency"
                />
              }
              @case ("lookup") {
                <atlas-lookup
                  controlId="docs-product"
                  ariaLabel="Product"
                  [rows]="products"
                  [columns]="productColumns"
                  [recordKey]="key"
                  [displayWith]="caption"
                  [(value)]="product"
                />
                <p>Selected ID: {{ product() || "None" }}</p>
              }
              @case ("reference") {
                @if (session.has("crm.customers.lookup")) {
                  <bqatlas-reference-lookup
                    controlId="docs-customer"
                    label="Customer"
                    resource="crm.customers"
                    [provider]="customers"
                    [columns]="customerColumns"
                    [recordKey]="key"
                    [displayWith]="caption"
                    [value]="customer()"
                    (recordSelected)="customer.set($event?.id || null)"
                  />
                  <p>
                    This example uses your connected customer records. Create
                    and edit actions save to the database.
                  </p>
                } @else {
                  <p>
                    Customer lookup permission is needed for this connected
                    example.
                  </p>
                }
              }
              @case ("multi") {
                <atlas-multi-select
                  controlId="docs-tags"
                  label="Account tags"
                  [options]="tags"
                  [(value)]="selectedTags"
                  [limit]="2"
                />
              }
              @case ("radio") {
                <atlas-radio-group
                  controlId="docs-kind"
                  label="Activity type"
                  [options]="kinds"
                  [(value)]="kind"
                />
              }
              @case ("checkbox") {
                <atlas-checkbox-group
                  controlId="docs-channel"
                  label="Contact channels"
                  [options]="channels"
                  [(value)]="selectedChannels"
                />
              }
              @case ("toggle") {
                <atlas-toggle
                  controlId="docs-active"
                  label="Available for new business"
                  [(checked)]="active"
                />
              }
              @case ("date") {
                <atlas-date-input
                  controlId="docs-due"
                  label="Due date"
                  [(value)]="date"
                />
              }
              @case ("decimal") {
                <bqatlas-decimal-input
                  controlId="docs-price"
                  ariaLabel="Unit price"
                  [(ngModel)]="price"
                  [scale]="4"
                  maximum="1000000000"
                />
                <p>String value: {{ price }}</p>
              }
              @case ("tabs") {
                <atlas-tabs label="Customer sections"
                  ><ng-template atlasTab="details" label="Details"
                    ><p>Contact details and preferences.</p></ng-template
                  ><ng-template atlasTab="sales" label="Sales"
                    ><p>
                      Quotes and opportunities stay connected to this customer.
                    </p></ng-template
                  ></atlas-tabs
                >
              }
              @case ("accordion") {
                <atlas-accordion-section label="Contact preferences"
                  ><p>
                    Preferred contact: email. Collapse and expand this section.
                  </p></atlas-accordion-section
                >
              }
              @case ("split") {
                <atlas-split-pane
                  primaryLabel="Customers"
                  secondaryLabel="Overview"
                  [ratio]="45"
                  ><div atlasSplitPrimary>
                    <p>Northwind Trading</p>
                    <p>Alpine Studio</p>
                  </div>
                  <div atlasSplitSecondary>
                    <h3>Customer overview</h3>
                    <p>Drag the separator or resize with the keyboard.</p>
                  </div></atlas-split-pane
                >
              }
              @case ("table") {
                <atlas-table
                  [rows]="products"
                  [columns]="productColumns"
                  keyField="id"
                  label="Products"
                  [filterable]="true"
                  [columnManage]="true"
                  [multiSort]="true"
                  [selectable]="true"
                />
              }
              @case ("dialog") {
                <button atlasButton variant="primary" (click)="open.set(true)">
                  Open dialog</button
                ><atlas-dialog title="Review account" [(open)]="open"
                  ><p>Check the customer details before continuing.</p>
                  <button atlasButton (click)="open.set(false)">
                    Done
                  </button></atlas-dialog
                >
              }
            }
          </div>
        </section>
        <h2>Usage</h2>
        <p class="docs-note">
          Add the listed components to your standalone component’s
          <code>imports</code>. Examples use Angular signals; ngModel examples
          also require <code>FormsModule</code>.
        </p>
        <pre><code>{{sourceImport()}}</code></pre>
        <h3>Component state</h3>
        <pre><code>{{doc().setup}}</code></pre>
        <h3>Template</h3>
        <pre><code>{{doc().template}}</code></pre>
        <h2>API & integration</h2>
        <p>{{ doc().api }}</p>
        <h2>Keyboard & accessibility</h2>
        <p>{{ doc().keyboard }}</p>
        <aside class="docs-note">
          Use stable, unique control IDs and explicit labels. Server-side
          validation and permissions remain authoritative. The full form
          showcase demonstrates these controls in larger workflows.
        </aside>
      </article>
    </div>
  </section>`,
})
export class ControlDocs {
  readonly allDocs = controlDocs;
  readonly article = viewChild<ElementRef<HTMLElement>>("article");
  select(id: string) {
    this.id.set(id);
    this.article()?.nativeElement.scrollTo({ top: 0 });
  }
  readonly workspace = inject(WorkspaceService);
  readonly session = inject(AtlasSession);
  readonly search = signal("");
  readonly id = signal<string>("lookup");
  readonly groups = [
    "Foundations",
    "Inputs",
    "Selection",
    "Layout",
    "Data",
    "Feedback",
    "Integration",
  ];
  readonly filtered = computed(() =>
    controlDocs.filter((d) =>
      (d.title + " " + d.description + " " + d.imports)
        .toLowerCase()
        .includes(this.search().toLowerCase()),
    ),
  );
  readonly doc = computed(() => controlDocs.find((d) => d.id === this.id())!);
  inGroup(group: string) {
    return this.filtered().filter((d) => d.group === group);
  }
  sourceImport() {
    return `import { ${this.doc().imports} } from '${["reference", "decimal"].includes(this.id()) ? "@bqatlas/angular" : "@bqatlas/ui"}';`;
  }
  showcase() {
    this.workspace.open({ screen: "bqatlas.showcase", data: {} });
  }
  count = signal(0);
  name = "Northwind Trading";
  notes = signal("Follow up after the product demonstration.");
  currency = "USD";
  currencies = [
    { value: "USD", label: "US Dollar" },
    { value: "EUR", label: "Euro" },
  ];
  products = [
    {
      id: "1",
      code: "CONSULT",
      name: "Implementation consulting",
      price: "125.00",
    },
    { id: "2", code: "TRAIN", name: "Team training", price: "500.00" },
    { id: "3", code: "SUPPORT", name: "Priority support", price: "250.00" },
  ];
  productColumns = [
    { key: "code" as const, label: "SKU" },
    { key: "name" as const, label: "Product" },
    { key: "price" as const, label: "Price", sortable: false },
  ];
  product = signal<string | null>(null);
  key = (r: { id: string }) => r.id;
  caption = (r: { name: string }) => r.name;
  customers = new RestLookupProvider<{
    id: string;
    code: string;
    name: string;
  }>(inject(AtlasApi), "/api/v1/crm/customers/lookup");
  customerColumns = [
    { key: "code" as const, label: "Code" },
    { key: "name" as const, label: "Customer" },
  ];
  customer = signal<string | null>(null);
  tags = [
    { value: "partner", label: "Partner" },
    { value: "priority", label: "Priority" },
    { value: "retail", label: "Retail" },
  ];
  selectedTags = signal<string[]>([]);
  kinds = [
    { value: "Call", label: "Call" },
    { value: "Meeting", label: "Meeting" },
  ];
  kind = signal<string | null>("Call");
  channels = [
    { value: "email", label: "Email" },
    { value: "phone", label: "Phone" },
  ];
  selectedChannels = signal<string[]>(["email"]);
  active = signal(true);
  date = signal("2026-10-01");
  price = "125.0000";
  open = signal(false);
}
