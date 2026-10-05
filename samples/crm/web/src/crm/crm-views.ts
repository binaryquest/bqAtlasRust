import {
  Component,
  ChangeDetectionStrategy,
  computed,
  inject,
  signal,
  effect,
  untracked,
  DestroyRef,
  viewChildren,
} from "@angular/core";
import { FormsModule } from "@angular/forms";
import {
  ATLAS_TASK,
  AtlasButton,
  AtlasInput,
  AtlasTable,
  AtlasTabs,
  AtlasTab,
  AtlasBadge,
  AtlasSplitPane,
  AtlasAccordionSection,
  AtlasLatestRequest,
  AtlasColumn,
  AtlasTableQuery,
} from "@bqatlas/ui";
import {
  AtlasApi,
  AtlasSession,
  CrudWorkspace,
  CrudFeature,
  RestResourceProvider,
} from "@bqatlas/angular";
import { QueryRequest } from "@bqatlas/contracts";
import {
  CrmCustomerField,
  CrmNotesField,
  CrmChoiceField,
  CrmActiveField,
  CrmDateField,
} from "./crm-fields";
export type CrmRow = {
  id: string;
  name: string;
  code?: string;
  email?: string;
  active?: boolean;
  customerId?: string;
  customerName?: string;
  stage?: string;
  amount?: string;
  currency?: string;
  owner?: string;
  expectedClose?: string;
  dueDate?: string;
  kind?: string;
  status?: string;
  unitPrice?: string;
  category?: string;
  notes?: string;
};
const columns: Record<string, AtlasColumn<CrmRow>[]> = {
  "crm.customers": [
    { key: "code", label: "Account code", width: 140 },
    { key: "name", label: "Customer" },
    { key: "email", label: "Email" },
    {
      key: "active",
      label: "Status",
      width: 90,
      format: (row) => (row.active ? "Active" : "Inactive"),
      badge: true,
      tone: (row) => (row.active ? "success" : "neutral"),
    },
  ],
  "engagement.products": [
    { key: "code", label: "SKU", width: 130 },
    { key: "name", label: "Product" },
    { key: "category", label: "Category" },
    { key: "unitPrice", label: "Unit price", align: "right", sortable: false },
    { key: "currency", label: "Currency" },
    {
      key: "active",
      label: "Status",
      format: (row) => (row.active ? "Active" : "Inactive"),
      badge: true,
      tone: (row) => (row.active ? "success" : "neutral"),
    },
  ],
  "engagement.opportunities": [
    { key: "name", label: "Opportunity" },
    { key: "customerName", label: "Customer", sortable: false },
    {
      key: "stage",
      label: "Stage",
      badge: true,
      tone: (row) =>
        row.stage === "Won"
          ? "success"
          : row.stage === "Lost"
            ? "neutral"
            : "info",
    },
    { key: "amount", label: "Expected value", align: "right", sortable: false },
    { key: "currency", label: "Currency" },
    { key: "expectedClose", label: "Expected close" },
    { key: "owner", label: "Owner" },
  ],
  "engagement.activities": [
    { key: "name", label: "Subject" },
    { key: "customerName", label: "Customer", sortable: false },
    { key: "kind", label: "Type" },
    { key: "dueDate", label: "Due date" },
    {
      key: "status",
      label: "Status",
      badge: true,
      tone: (row) =>
        row.status === "Done"
          ? "success"
          : row.status === "Planned"
            ? "info"
            : "neutral",
    },
    { key: "owner", label: "Owner" },
  ],
};
@Component({
  selector: "app-crm-workbench",
  imports: [
    FormsModule,
    AtlasButton,
    AtlasInput,
    AtlasTable,
    AtlasBadge,
    AtlasSplitPane,
    AtlasAccordionSection,
    AtlasTabs,
    AtlasTab,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `<section class="crm-workbench">
    <header class="crm-heading">
      <div>
        <p class="eyebrow">CUSTOMER RELATIONSHIPS</p>
        <h2>{{ title }}</h2>
        <p>{{ subtitle }}</p>
      </div>
      <div class="crm-actions">
        @if (canCreate()) {
          <button atlasButton variant="primary" (click)="create()">
            New {{ singular }}
          </button>
        }
        <button atlasButton (click)="reload()">Refresh</button>
      </div>
    </header>
    <nav class="crm-navigation" aria-label="CRM sections">
      @for (item of navigation; track item.id) {
        @if (session.has(item.id + ".read")) {
          <button
            atlasButton
            [variant]="resource === item.id ? 'primary' : 'ghost'"
            (click)="crud.openList(item.id)"
          >
            {{ item.label }}
          </button>
        }
      }
      @if (session.has("sales.quotes.read")) {
        <button
          atlasButton
          variant="ghost"
          (click)="crud.openList('sales.quotes')"
        >
          Sales quotes ↗
        </button>
      }
    </nav>
    @if (customerFilter()) {
      <div class="crm-filter-banner">
        Showing work for {{ customerFilter()!.name }}
        <button atlasButton (click)="clearFilter()">All customers</button>
      </div>
    }
    @if (resource === "engagement.opportunities") {
      <atlas-tabs label="Pipeline views" [(selected)]="pipelineView"
        ><ng-template atlasTab="board" label="Pipeline board"
          ><div class="crm-board-tools">
            <input
              atlasInput
              type="search"
              aria-label="Search pipeline"
              placeholder="Find opportunity, customer or owner…"
              [ngModel]="request.search"
              (ngModelChange)="search($event)"
            /><span
              >{{ total() }} opportunities · Page {{ request.page! + 1 }} ·
              {{ rows().length }} shown</span
            ><button
              atlasButton
              [disabled]="!request.page || loading()"
              (click)="page(-1)"
            >
              Previous</button
            ><button
              atlasButton
              [disabled]="
                (request.page! + 1) * request.pageSize! >= total() || loading()
              "
              (click)="page(1)"
            >
              Next
            </button>
          </div>
          @if (error()) {
            <p class="atlas-alert error" role="alert">{{ error() }}</p>
          }
          @if (loading()) {
            <p role="status">Loading pipeline…</p>
          }
          <div class="crm-pipeline">
            @for (stage of stages; track stage) {
              <section class="crm-stage" [attr.aria-label]="stage">
                <header>
                  <strong>{{ stage }}</strong
                  ><atlas-badge>{{ inStage(stage).length }}</atlas-badge>
                </header>
                @for (row of inStage(stage); track row.id) {
                  <button type="button" class="crm-deal" (click)="edit(row)">
                    <span>{{ row.customerName }}</span
                    ><strong>{{ row.name }}</strong
                    ><b>{{ row.currency }} {{ row.amount }}</b
                    ><small>{{ row.owner }} · {{ row.expectedClose }}</small>
                  </button>
                } @empty {
                  <p class="crm-empty-stage">
                    No {{ stage.toLowerCase() }} opportunities on this page.
                  </p>
                }
              </section>
            }
          </div></ng-template
        >
        <ng-template atlasTab="list" label="All opportunities">
          @if (pipelineView() === "list") {
            <atlas-table
              [rows]="rows()"
              [columns]="tableColumns"
              keyField="id"
              label="opportunities"
              [server]="true"
              [total]="total()"
              [pageSize]="25"
              [loading]="loading()"
              [error]="error()"
              [filterable]="true"
              [columnToggle]="true"
              (queryChange)="query($event)"
              (rowActivated)="edit($event)"
              (retry)="reload()"
            />
          }</ng-template
      ></atlas-tabs>
    } @else if (resource === "crm.customers") {
      <atlas-split-pane
        primaryLabel="Customer directory"
        secondaryLabel="Customer overview"
        [ratio]="64"
        [minimum]="35"
        [maximum]="75"
      >
        <div atlasSplitPrimary>
          <atlas-table
            [rows]="rows()"
            [columns]="tableColumns"
            keyField="id"
            label="customers"
            [server]="true"
            [total]="total()"
            [pageSize]="25"
            [loading]="loading()"
            [error]="error()"
            [filterable]="true"
            [columnToggle]="true"
            (queryChange)="query($event)"
            (rowActivated)="selectCustomer($event)"
            (retry)="reload()"
          />
        </div>
        <aside atlasSplitSecondary class="crm-customer-overview">
          @if (selected(); as customer) {
            <span class="crm-avatar">{{
              customer.name.slice(0, 2).toUpperCase()
            }}</span>
            <h2>{{ customer.name }}</h2>
            <p>
              {{ customer.code }} ·
              {{ customer.active ? "Active account" : "Inactive account" }}
            </p>
            <p>{{ customer.email || "No email provided" }}</p>
            <div class="crm-actions">
              <button atlasButton variant="primary" (click)="edit(customer)">
                Open customer
              </button>
              @if (
                customer.active &&
                session.has("sales.quotes.write") &&
                session.has("sales.quotes.read") &&
                session.has("crm.customers.lookup")
              ) {
                <button atlasButton (click)="newQuote(customer)">
                  New quote
                </button>
              }
            </div>
            <atlas-accordion-section label="Sales & follow-ups"
              ><p>Open linked records or start the next conversation.</p>
              <div class="crm-quick-links">
                @if (session.has("engagement.opportunities.read")) {
                  <button
                    atlasButton
                    (click)="related('engagement.opportunities', customer)"
                  >
                    View pipeline →
                  </button>
                }
                @if (session.has("engagement.activities.read")) {
                  <button
                    atlasButton
                    (click)="related('engagement.activities', customer)"
                  >
                    View activities →
                  </button>
                }
                @if (
                  session.has("engagement.activities.write") &&
                  session.has("engagement.activities.read") &&
                  session.has("crm.customers.lookup")
                ) {
                  <button
                    atlasButton
                    (click)="createRelated('engagement.activities', customer)"
                  >
                    Schedule follow-up
                  </button>
                }
                @if (
                  session.has("engagement.opportunities.write") &&
                  session.has("engagement.opportunities.read") &&
                  session.has("crm.customers.lookup")
                ) {
                  <button
                    atlasButton
                    (click)="
                      createRelated('engagement.opportunities', customer)
                    "
                  >
                    Add opportunity
                  </button>
                }
              </div></atlas-accordion-section
            >
          } @else {
            <div class="crm-empty">
              <h3>A complete view of your customer</h3>
              <p>
                Open a row to see the account and start a quote, opportunity or
                follow-up.
              </p>
            </div>
          }
        </aside></atlas-split-pane
      >
    } @else {
      <atlas-table
        [rows]="rows()"
        [columns]="tableColumns"
        keyField="id"
        [label]="title"
        [server]="true"
        [total]="total()"
        [pageSize]="25"
        [loading]="loading()"
        [error]="error()"
        [filterable]="true"
        [columnToggle]="true"
        [columnManage]="true"
        [multiSort]="true"
        (queryChange)="query($event)"
        (rowActivated)="edit($event)"
        (retry)="reload()"
      />
      <p class="crm-footnote">
        {{
          resource === "engagement.products"
            ? "Catalog prices are copied into quote lines. Existing quotes keep their saved description and price."
            : "Open a follow-up to change its date, owner or completion status. Dates are local calendar dates."
        }}
      </p>
    }
  </section>`,
  styleUrl: "crm.css",
})
export class CrmWorkbench {
  readonly task = inject(ATLAS_TASK);
  readonly crud = inject(CrudWorkspace);
  readonly session = inject(AtlasSession);
  readonly api = inject(AtlasApi);
  readonly resource = (this.task.data() as { resource: string }).resource;
  readonly title = this.crud.feature(this.resource).title;
  readonly tableColumns = columns[this.resource];
  readonly singular =
    this.resource === "crm.customers"
      ? "customer"
      : this.resource.endsWith("products")
        ? "product"
        : this.resource.endsWith("activities")
          ? "activity"
          : "opportunity";
  readonly subtitle =
    this.resource === "crm.customers"
      ? "Know your customers. Keep every conversation connected."
      : this.resource.endsWith("products")
        ? "A shared catalog for consistent sales quotes."
        : this.resource.endsWith("activities")
          ? "Plan the next step and keep your promises."
          : "From first conversation to a won relationship.";
  readonly navigation = [
    { id: "crm.customers", label: "Customers" },
    { id: "engagement.opportunities", label: "Pipeline" },
    { id: "engagement.activities", label: "Activities" },
    { id: "engagement.products", label: "Products" },
  ];
  readonly stages = ["Lead", "Qualified", "Proposal", "Won", "Lost"];
  readonly rows = signal<CrmRow[]>([]);
  readonly total = signal(0);
  readonly loading = signal(false);
  readonly error = signal("");
  readonly selected = signal<CrmRow | null>(null);
  readonly customerFilter = computed(
    () => (this.task.data() as { customer?: CrmRow }).customer,
  );
  readonly provider = new RestResourceProvider<CrmRow>(
    this.api,
    this.crud.descriptor(this.resource).endpoint,
  );
  private readonly latest = new AtlasLatestRequest();
  private timer?: ReturnType<typeof setTimeout>;
  request: QueryRequest = { page: 0, pageSize: 25, search: "" };
  readonly pipelineView = signal("board");
  readonly tables = viewChildren(AtlasTable);
  private previousContext = "";
  constructor() {
    effect(() => {
      this.crud.revision();
      const context =
        (this.customerFilter()?.id || "") + ":" + this.pipelineView();
      untracked(() => {
        if (context !== this.previousContext) {
          this.previousContext = context;
          this.request = { ...this.request, page: 0 };
          for (const table of this.tables()) table.page.set(0);
        }
        void this.reload();
      });
    });
    inject(DestroyRef).onDestroy(() => {
      this.latest.cancel();
      clearTimeout(this.timer);
    });
  }
  canCreate() {
    return (
      this.session.has(this.crud.feature(this.resource).writePermission) &&
      (this.resource === "crm.customers" ||
        this.resource.endsWith("products") ||
        this.session.has("crm.customers.lookup"))
    );
  }
  inStage(stage: string) {
    return this.rows().filter((row) => row.stage === stage);
  }
  query(value: AtlasTableQuery) {
    this.request = {
      ...this.request,
      page: value.page,
      pageSize: value.pageSize,
      search: value.search,
      sort: value.sort.map((s) => ({ field: s.key, direction: s.direction })),
    };
    void this.reload();
  }
  search(value: string) {
    this.request = { ...this.request, search: value, page: 0 };
    this.latest.cancel();
    clearTimeout(this.timer);
    this.timer = setTimeout(() => void this.reload(), 200);
  }
  page(delta: number) {
    this.request = { ...this.request, page: this.request.page! + delta };
    void this.reload();
  }
  reload() {
    this.loading.set(true);
    this.error.set("");
    const filter = this.customerFilter();
    return this.latest.run(
      (signal) =>
        this.provider.query(
          {
            ...this.request,
            filters: filter
              ? [{ field: "customerId", operator: "eq", value: filter.id }]
              : [],
          },
          signal,
        ),
      (result) => {
        this.rows.set(result.items);
        this.total.set(result.total);
        this.loading.set(false);
        const old = this.selected();
        if (old)
          this.selected.set(result.items.find((r) => r.id === old.id) || null);
      },
      (error) => {
        this.error.set(
          error instanceof Error ? error.message : "Unable to load records.",
        );
        this.loading.set(false);
      },
    );
  }
  create() {
    this.crud.openEditor(this.resource, undefined, this.task.id, {
      owner: this.session.info()?.name || "",
      ...this.customerDefaults(),
    });
  }
  customerDefaults() {
    const customer = this.customerFilter();
    return customer
      ? { customerId: customer.id, customerName: customer.name }
      : {};
  }
  edit(row: CrmRow) {
    this.crud.openEditor(this.resource, row.id, this.task.id);
  }
  selectCustomer(row: CrmRow) {
    this.selected.set(row);
  }
  newQuote(row: CrmRow) {
    this.crud.openEditor("sales.quotes", undefined, this.task.id, {
      customerId: row.id,
      customerCode: row.code,
      customerName: row.name,
    });
  }
  related(resource: string, customer: CrmRow) {
    this.crud.openList(resource);
    const task = this.workspace
      .tasks()
      .find((t) => t.screen === resource + ".list");
    if (task) task.data.set({ ...(task.data() as object), customer });
  }
  readonly workspace = inject(WorkspaceService);
  clearFilter() {
    this.task.data.set({ resource: this.resource });
  }
  createRelated(resource: string, customer: CrmRow) {
    this.crud.openEditor(resource, undefined, this.task.id, {
      customerId: customer.id,
      customerName: customer.name,
      owner: this.session.info()?.name || "",
    });
  }
}
import { WorkspaceService } from "@bqatlas/ui";
const today = () => {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
};
export const engagementFeatures: CrudFeature[] = [
  {
    resource: "engagement.products",
    title: "Products",
    icon: "box",
    writePermission: "engagement.products.write",
    deletePermission: "engagement.products.delete",
    defaults: {
      code: "",
      name: "",
      category: "Service",
      unitPrice: "0",
      currency: "USD",
      active: true,
    },
    listComponent: CrmWorkbench,
    fieldRenderers: { category: CrmChoiceField, active: CrmActiveField },
  },
  {
    resource: "engagement.opportunities",
    title: "Pipeline",
    icon: "activity",
    writePermission: "engagement.opportunities.write",
    deletePermission: "engagement.opportunities.delete",
    defaults: {
      name: "",
      customerId: "",
      stage: "Lead",
      amount: "0",
      currency: "USD",
      expectedClose: today(),
      owner: "",
      notes: "",
    },
    listComponent: CrmWorkbench,
    fieldRenderers: {
      customerId: CrmCustomerField,
      notes: CrmNotesField,
      stage: CrmChoiceField,
      expectedClose: CrmDateField,
    },
  },
  {
    resource: "engagement.activities",
    title: "Activities",
    icon: "activity",
    writePermission: "engagement.activities.write",
    deletePermission: "engagement.activities.delete",
    defaults: {
      name: "",
      customerId: "",
      kind: "Call",
      dueDate: today(),
      status: "Planned",
      owner: "",
      notes: "",
    },
    listComponent: CrmWorkbench,
    fieldRenderers: {
      customerId: CrmCustomerField,
      notes: CrmNotesField,
      kind: CrmChoiceField,
      status: CrmChoiceField,
      dueDate: CrmDateField,
    },
  },
];
