import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  inject,
  signal,
} from "@angular/core";
import { JsonPipe } from "@angular/common";
import { form, FormField, required } from "@angular/forms/signals";
import {
  AtlasBadge,
  AtlasButton,
  AtlasColumn,
  AtlasLatestRequest,
  AtlasListTemplate,
  AtlasListView,
  AtlasPanel,
  AtlasRowDetail,
  AtlasTable,
  AtlasTableQuery,
  AtlasTree,
  AtlasTreeNode,
  AtlasTreeSelect,
  filterCollection,
  matchesColumnFilters,
  pageCollection,
  sortByColumns,
} from "@bqatlas/ui";
interface StockRecord {
  id: string;
  product: string;
  warehouse: string;
  category: string;
  quantity: number;
  price: number;
}
@Component({
  selector: "demo-collections",
  imports: [
    JsonPipe,
    FormField,
    AtlasBadge,
    AtlasButton,
    AtlasPanel,
    AtlasTable,
    AtlasRowDetail,
    AtlasListView,
    AtlasListTemplate,
    AtlasTree,
    AtlasTreeSelect,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="lab-section-intro">
      <div>
        <h3>Collections that work together</h3>
        <p>
          Browse stock, inspect records, and choose a warehouse from its
          hierarchy.
        </p>
      </div>
      <atlas-badge tone="info">Interactive sample</atlas-badge>
    </div>
    <atlas-panel title="Inventory explorer">
      <p class="atlas-muted">
        Filter individual columns. Enable “Add sort levels” for ordered sorting.
        Use Columns to resize, reorder, or pin; drag a header edge to resize.
      </p>
      <div class="lab-form-actions">
        <button
          atlasButton
          [attr.aria-pressed]="remote()"
          (click)="toggleRemote()"
        >
          {{ remote() ? "Use local data" : "Simulate server paging" }}</button
        ><button
          atlasButton
          [disabled]="!remote() || loading()"
          (click)="simulateFailure()"
        >
          Simulate request failure</button
        ><span role="status"
          >{{ remote() ? "Dummy API · 350 ms latency" : "Local collection" }} ·
          {{ selectedKeys().length }} selected</span
        >
      </div>
      <atlas-table
        [rows]="remote() ? rows() : stock"
        [columns]="columns"
        keyField="id"
        label="Inventory stock"
        [pageSize]="5"
        [server]="remote()"
        [total]="total()"
        [loading]="remote() && loading()"
        [error]="remote() ? error() : ''"
        [filterable]="true"
        [selectable]="true"
        [columnToggle]="true"
        [columnManage]="true"
        [columnFilters]="true"
        [multiSort]="true"
        [(selectedKeys)]="selectedKeys"
        (queryChange)="request($event)"
        (retry)="request(lastQuery())"
        (rowActivated)="opened.set($event)"
      >
        <ng-template atlasRowDetail [atlasRowDetailOf]="stock" let-record
          ><strong>{{ record.product }} · {{ record.id }}</strong>
          <p>
            {{ record.warehouse }} / {{ record.category }} ·
            {{ record.quantity }} units available. Unit price:
            {{ record.price }} USD.
          </p>
          <p class="atlas-muted">
            Dummy stock record. Editing and stock adjustments arrive in the ERP
            editing phase.
          </p></ng-template
        >
      </atlas-table>
      @if (opened(); as row) {
        <p role="status">Opened {{ row.id }} — {{ row.product }}</p>
      }
    </atlas-panel>
    <div class="collection-demo-grid">
      <atlas-panel title="Warehouse hierarchy"
        ><atlas-tree
          [nodes]="warehouses"
          label="Warehouses"
          [(selectedId)]="warehouse"
          [expandedIds]="['north']"
        />
        <p class="atlas-muted">
          Selected ID: {{ warehouse() || "None" }}. Arrow keys navigate; Enter
          selects.
        </p></atlas-panel
      >
      <atlas-panel title="Choose a receiving location">
        <p class="atlas-muted">
          A Signal Forms field stores the location ID. The inline picker stays
          inside narrow application windows.
        </p>
        <atlas-tree-select
          [nodes]="warehouses"
          label="Receiving location"
          [formField]="locationForm.location"
        />
        <div class="lab-form-actions">
          <button atlasButton (click)="validate()">Validate location</button
          ><button atlasButton (click)="reset()">Reset location</button>
        </div>
        @if (
          locationForm.location().touched() && locationForm.location().invalid()
        ) {
          <p role="alert">Choose a receiving location.</p>
        }
        <pre class="lab-value-preview">{{ location() | json }}</pre>
      </atlas-panel>
    </div>
    <atlas-panel title="List item templates">
      <p class="atlas-muted">
        Application-specific presentation with the same search, keyboard
        navigation, and selection behavior.
      </p>
      <atlas-list-view
        [items]="suppliers"
        label="Preferred suppliers"
        [pageSize]="3"
      >
        <ng-template atlasListTemplate let-item let-selected="selected"
          ><span class="supplier-mark">{{ item.id }}</span
          ><span class="atlas-list-item-content"
            ><strong>{{ item.title }}</strong
            ><small>{{ item.description }}</small
            ><span class="atlas-list-meta">{{ item.meta }}</span></span
          ><atlas-badge [tone]="selected ? 'info' : 'success'">{{
            selected ? "Selected" : "Approved"
          }}</atlas-badge></ng-template
        >
      </atlas-list-view>
    </atlas-panel>
    <atlas-panel title="API request preview">
      <pre class="lab-value-preview">{{ lastQuery() | json }}</pre>
      <p class="atlas-muted">
        Queries are serializable. A new request cancels the previous one; stale
        responses cannot replace current records. Selection uses stable IDs
        across pages.
      </p></atlas-panel
    >
  `,
})
export class CollectionExamples {
  readonly remote = signal(false);
  readonly loading = signal(false);
  readonly error = signal("");
  readonly rows = signal<StockRecord[]>([]);
  readonly total = signal(0);
  readonly selectedKeys = signal<string[]>([]);
  readonly opened = signal<StockRecord | null>(null);
  readonly warehouse = signal<string | null>(null);
  readonly location = signal<{ location: string | null }>({ location: null });
  readonly locationForm = form(this.location, (schema) =>
    required(schema.location),
  );
  readonly lastQuery = signal<AtlasTableQuery>({
    page: 0,
    pageSize: 5,
    search: "",
    filters: {},
    sort: [],
  });
  private failNext = false;
  private latest = new AtlasLatestRequest();
  readonly stock: StockRecord[] = Array.from({ length: 48 }, (_, i) => ({
    id: `SKU-${String(i + 1).padStart(3, "0")}`,
    product: [
      "Studio desk",
      "Task chair",
      "Desk lamp",
      "Storage shelf",
      "Monitor arm",
      "Desk mat",
    ][i % 6],
    warehouse: ["Dhaka", "London", "Berlin"][Math.floor(i / 6) % 3],
    category: i % 3 === 0 ? "Furniture" : "Accessories",
    quantity: (i * 17 + 12) % 120,
    price: [425, 189, 65, 210, 89, 24][i % 6],
  }));
  readonly columns: AtlasColumn<StockRecord>[] = [
    { key: "id", label: "SKU", width: 140 },
    { key: "product", label: "Product", width: 210 },
    { key: "warehouse", label: "Warehouse", width: 160 },
    { key: "category", label: "Category", width: 160 },
    { key: "quantity", label: "Available", align: "right", width: 130 },
    {
      key: "price",
      label: "Unit price",
      align: "right",
      width: 140,
      format: (row) =>
        new Intl.NumberFormat("en-US", {
          style: "currency",
          currency: "USD",
        }).format(row.price),
    },
  ];
  readonly warehouses: AtlasTreeNode[] = [
    {
      id: "north",
      label: "North region",
      children: [
        {
          id: "dhaka",
          label: "Dhaka distribution",
          children: [
            {
              id: "dhaka-a",
              label: "Zone A",
              description: "General receiving",
            },
            { id: "dhaka-b", label: "Zone B", description: "Cold storage" },
          ],
        },
        {
          id: "london",
          label: "London depot",
          children: [{ id: "london-a", label: "Main floor" }],
        },
      ],
    },
    {
      id: "central",
      label: "Central region",
      children: [
        {
          id: "berlin",
          label: "Berlin hub",
          children: [
            { id: "berlin-a", label: "Receiving bay" },
            {
              id: "berlin-old",
              label: "Legacy bay",
              disabled: true,
              description: "Closed for renovation",
            },
          ],
        },
      ],
    },
  ];
  readonly suppliers = [
    {
      id: "NS",
      title: "Northstar Supply",
      description: "Office furniture · Dhaka",
      meta: "Lead time: 4 days · Terms: Net 30",
    },
    {
      id: "MS",
      title: "Meridian Studio",
      description: "Lighting · London",
      meta: "Lead time: 7 days · Terms: Net 15",
    },
    {
      id: "AC",
      title: "Acme Components",
      description: "Accessories · Berlin",
      meta: "Lead time: 3 days · Terms: Net 30",
    },
  ];
  constructor() {
    inject(DestroyRef).onDestroy(() => this.latest.cancel());
  }
  toggleRemote() {
    this.remote.update((value) => !value);
    this.latest.cancel();
    this.loading.set(false);
    this.error.set("");
    if (this.remote()) this.request(this.lastQuery());
  }
  simulateFailure() {
    this.failNext = true;
    this.request(this.lastQuery());
  }
  request(query: AtlasTableQuery) {
    this.lastQuery.set(query);
    if (!this.remote()) return;
    this.loading.set(true);
    this.error.set("");
    const fail = this.failNext;
    this.failNext = false;
    void this.latest.run(
      async (signal) => {
        await new Promise<void>((resolve, reject) => {
          const timer = setTimeout(() => {
            signal.removeEventListener("abort", abort);
            resolve();
          }, 350);
          const abort = () => {
            clearTimeout(timer);
            reject(new Error("Aborted"));
          };
          signal.addEventListener("abort", abort, { once: true });
        });
        if (fail)
          throw new Error(
            "The dummy inventory service is unavailable. Retry this request.",
          );
        const value = (row: StockRecord, key: string) => {
          const column = this.columns.find((c) => c.key === key);
          return column?.format
            ? column.format(row)
            : String(row[key as keyof StockRecord] ?? "");
        };
        const filtered = filterCollection(
          this.stock.filter((row) =>
            matchesColumnFilters(row, query.filters, value),
          ),
          query.search,
          (row) => this.columns.map((c) => value(row, c.key)).join(" "),
        );
        const sorted = sortByColumns(
          filtered,
          query.sort,
          (row, key) => row[key as keyof StockRecord],
        );
        return pageCollection(sorted, query.page, query.pageSize);
      },
      (result) => {
        this.rows.set(result.items);
        this.total.set(result.total);
        this.loading.set(false);
      },
      (error) => {
        this.error.set(
          error instanceof Error ? error.message : "Request failed.",
        );
        this.loading.set(false);
      },
    );
  }
  validate() {
    this.locationForm.location().markAsTouched();
  }
  reset() {
    this.locationForm().reset({ location: null });
  }
}
