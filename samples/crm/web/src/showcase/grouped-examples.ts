import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  inject,
  signal,
  viewChild,
} from "@angular/core";
import {
  AtlasButton,
  AtlasInput,
  AtlasPanel,
  AtlasTable,
  AtlasColumn,
  AtlasSummary,
  AtlasServerSummary,
  AtlasTableQuery,
  AtlasLatestRequest,
  atlasSummaryQueryKey,
  atlasSumDecimal,
  filterCollection,
  matchesColumnFilters,
  sortByColumns,
  pageCollection,
} from "@bqatlas/ui";
interface Order {
  id: string;
  customer: string;
  status: string;
  date: string;
  net: string;
  tax: string;
}
@Component({
  selector: "demo-grouped-tables",
  imports: [AtlasButton, AtlasInput, AtlasPanel, AtlasTable],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="lab-section-intro">
      <div>
        <h3>Order register</h3>
        <p>
          Grouped orders, exact USD totals and headers that follow visible
          columns.
        </p>
      </div>
    </div>
    <atlas-panel title="Sales orders · USD">
      <div panelToolbar class="lab-form-actions">
        <label
          >Group by
          <select
            atlasInput
            [value]="group() || ''"
            (change)="group.set($any($event.target).value || null)"
          >
            <option value="customer">Customer</option>
            <option value="status">Status</option>
            <option value="">No grouping</option>
          </select></label
        >
        <label
          ><input
            type="checkbox"
            [checked]="remote()"
            (change)="setRemote($any($event.target).checked)"
          />
          Simulate server paging</label
        >
        <label
          ><input
            type="checkbox"
            [checked]="failNext()"
            (change)="failNext.set($any($event.target).checked)"
          />
          Fail next request</label
        >
        <button atlasButton (click)="reset()">Reset register</button>
      </div>
      <p class="atlas-muted">
        {{
          remote()
            ? "Local server simulation: delayed requests, cancellation, retry and whole-query aggregates."
            : "Local data: group subtotals cover the current page; filtered totals cover every matching order."
        }}
        Collapse a group without changing totals or selection.
      </p>
      <atlas-table
        [rows]="remote() ? remoteRows() : orders"
        [columns]="columns"
        keyField="id"
        label="Grouped order register"
        [pageSize]="6"
        [groupBy]="group()"
        [summaries]="summaries"
        [serverSummary]="remoteSummary()"
        [server]="remote()"
        [total]="total()"
        [loading]="remote() && loading()"
        [error]="remote() ? error() : ''"
        (retry)="load(lastQuery)"
        (queryChange)="queryChanged($event)"
        [filterable]="true"
        [columnFilters]="true"
        [columnManage]="true"
        [columnToggle]="true"
        [multiSort]="true"
        [selectable]="true"
        (rowActivated)="opened.set($event.id)"
      />
      @if (opened()) {
        <p role="status">
          Selected sample order {{ opened() }}. This register does not edit
          business records.
        </p>
      }
    </atlas-panel>
  `,
})
export class GroupedExamples {
  readonly table = viewChild(AtlasTable);
  readonly group = signal<"customer" | "status" | null>("customer");
  readonly remote = signal(false);
  readonly remoteRows = signal<Order[]>([]);
  readonly remoteSummary = signal<AtlasServerSummary | null>(null);
  readonly total = signal(0);
  readonly loading = signal(false);
  readonly error = signal("");
  readonly failNext = signal(false);
  readonly opened = signal("");
  readonly request = new AtlasLatestRequest();
  lastQuery: AtlasTableQuery = {
    page: 0,
    pageSize: 6,
    search: "",
    filters: {},
    sort: [],
  };
  readonly orders: Order[] = Array.from({ length: 18 }, (_, index) => ({
    id: `SO-${1041 + index}`,
    customer: ["Northstar Supply", "Meridian Studio", "Cedar Industries"][
      index % 3
    ],
    status: ["Approved", "Draft", "Shipped"][Math.floor(index / 3) % 3],
    date: `2026-09-${String(index + 1).padStart(2, "0")}`,
    net: `${100 + index * 17}.${index % 2 ? "20" : "10"}`,
    tax: `${10 + index}.05`,
  }));
  readonly columns: AtlasColumn<Order>[] = [
    { key: "id", label: "Order", group: "Order details", width: 120 },
    { key: "customer", label: "Customer", group: "Order details", width: 210 },
    {
      key: "status",
      label: "Status",
      group: "Fulfilment",
      badge: true,
      width: 120,
    },
    { key: "date", label: "Order date", group: "Fulfilment", width: 130 },
    {
      key: "net",
      label: "Net · USD",
      group: "Amounts · USD",
      align: "right",
      width: 120,
    },
    {
      key: "tax",
      label: "Tax · USD",
      group: "Amounts · USD",
      align: "right",
      width: 120,
    },
  ];
  readonly summaries: AtlasSummary<Order>[] = [
    {
      key: "net",
      label: "Net USD",
      aggregate: (rows) => atlasSumDecimal(rows.map((row) => row.net)),
    },
    {
      key: "tax",
      label: "Tax USD",
      aggregate: (rows) => atlasSumDecimal(rows.map((row) => row.tax)),
    },
  ];
  constructor() {
    inject(DestroyRef).onDestroy(() => this.request.cancel());
  }
  queryChanged(query: AtlasTableQuery) {
    this.lastQuery = query;
    if (this.remote()) this.load(query);
  }
  setRemote(value: boolean) {
    this.remote.set(value);
    this.request.cancel();
    if (value) this.load(this.lastQuery);
  }
  reset() {
    this.request.cancel();
    this.remote.set(false);
    this.failNext.set(false);
    this.group.set("customer");
    this.opened.set("");
    const table = this.table();
    if (table) {
      table.query.set("");
      table.filters.set({});
      table.sorts.set([]);
      table.page.set(0);
      table.selectedKeys.set([]);
      table.collapsedGroups.set([]);
      table.hiddenColumns.set([]);
      table.order.set([]);
      table.pinned.set([]);
      table.widths.set({});
    }
  }
  load(query: AtlasTableQuery) {
    this.loading.set(true);
    this.error.set("");
    this.remoteSummary.set(null);
    const fail = this.failNext();
    this.failNext.set(false);
    void this.request.run(
      async () => {
        await new Promise((resolve) => setTimeout(resolve, 450));
        if (fail)
          throw new Error(
            "Simulated connection failure. Your filters are retained.",
          );
        const filtered = filterCollection(
          this.orders.filter((row) =>
            matchesColumnFilters(row, query.filters, (item, key) =>
              String(item[key as keyof Order]),
            ),
          ),
          query.search,
          (row) => Object.values(row).join(" "),
        );
        return {
          rows: pageCollection(
            sortByColumns(
              filtered,
              query.sort,
              (row, key) => row[key as keyof Order],
            ),
            query.page,
            query.pageSize,
          ).items,
          total: filtered.length,
          summary: {
            scope: "query" as const,
            queryKey: atlasSummaryQueryKey(query),
            values: Object.fromEntries(
              this.summaries.map((summary) => [
                summary.key,
                summary.aggregate(filtered),
              ]),
            ),
          },
        };
      },
      (result) => {
        this.remoteRows.set(result.rows);
        this.total.set(result.total);
        this.remoteSummary.set(result.summary);
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
}
