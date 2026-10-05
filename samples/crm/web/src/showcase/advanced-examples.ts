import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  computed,
  effect,
  inject,
  signal,
  viewChild,
} from "@angular/core";
import { FormField, disabled, form, readonly } from "@angular/forms/signals";
import {
  ATLAS_TASK,
  AtlasButton,
  AtlasInput,
  AtlasPivotTable,
  AtlasChart,
  AtlasDashboard,
  AtlasDashboardPanel,
  AtlasCalendar,
  AtlasRichText,
  AtlasVirtualGrid,
  WorkspaceService,
  atlasPivot,
} from "@bqatlas/ui";
import type { AtlasColumn } from "@bqatlas/ui";
import { PlanningStore } from "./planning-store";
interface Sale {
  id: string;
  region: string;
  month: string;
  channel: string;
  amount: string;
}
const sales: Sale[] = Array.from({ length: 72 }, (_, index) => ({
  id: `SO-${1001 + index}`,
  region: ["Dhaka", "Chattogram", "Sylhet"][index % 3],
  month: ["2026-07", "2026-08", "2026-09"][Math.floor(index / 3) % 3],
  channel: index % 2 ? "Wholesale" : "Retail",
  amount: (
    ((125000 + index * 1731) * (index % 17 === 0 ? -1 : 1)) /
    100
  ).toFixed(2),
}));
@Component({
  selector: "demo-analytics",
  imports: [
    AtlasButton,
    AtlasInput,
    AtlasPivotTable,
    AtlasChart,
    AtlasDashboard,
    AtlasDashboardPanel,
  ],
  styleUrl: "./advanced-examples.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: ` <section class="advanced-demo">
    <header>
      <div>
        <h3>Sales analytics & dashboards</h3>
        <p>
          Exact decimal totals from 72 sample orders, including credit
          adjustments.
        </p>
      </div>
      <button atlasButton (click)="order.set([])">Reset panel layout</button>
    </header>
    <div class="advanced-tools">
      <label
        >Sales channel<select
          atlasInput
          [value]="channel()"
          (change)="channel.set($any($event.target).value); drill.set(null)"
        >
          <option value="">All channels</option>
          <option>Retail</option>
          <option>Wholesale</option>
        </select></label
      ><label
        >Row dimension<select
          atlasInput
          [value]="dimension()"
          (change)="dimension.set($any($event.target).value); drill.set(null)"
        >
          <option value="region">Region</option>
          <option value="channel">Channel</option>
        </select></label
      ><label
        >Chart type<select
          atlasInput
          [value]="chartType()"
          (change)="chartType.set($any($event.target).value)"
        >
          <option value="bar">Bar</option>
          <option value="line">Line</option>
        </select></label
      >
    </div>
    <atlas-dashboard [(order)]="order">
      <ng-template atlasDashboardPanel="overview" title="Sales snapshot"
        ><div class="advanced-kpis">
          <div>
            <small>NET SALES · USD</small
            ><strong>{{ pivot().grandTotal }}</strong>
          </div>
          <div>
            <small>FILTERED ORDERS</small
            ><strong>{{ filtered().length }}</strong>
          </div>
        </div>
        <p class="atlas-muted">
          Includes credits. Filters and panel positions are local preferences
          retained while this task is open.
        </p></ng-template
      >
      <ng-template atlasDashboardPanel="chart" title="Sales by group"
        ><atlas-chart
          [points]="points()"
          [type]="chartType()"
          label="Net sales by group · USD"
          (pointSelected)="drill.set({ row: $event.id, column: '' })"
      /></ng-template>
      <ng-template atlasDashboardPanel="pivot" title="Monthly pivot"
        ><atlas-pivot-table
          [records]="filtered()"
          [row]="rowSelector()"
          [column]="column"
          [measure]="measure"
          label="Net sales · USD"
          [rowLabel]="dimension()"
          (cellSelected)="drill.set($event)"
      /></ng-template>
    </atlas-dashboard>
    @if (drill(); as cell) {
      <section class="advanced-card">
        <header>
          <h4>Orders: {{ cell.row }} {{ cell.column }}</h4>
          <button atlasButton (click)="drill.set(null)">
            Close drill-down
          </button>
        </header>
        <div class="atlas-table-scroll">
          <table class="atlas-table">
            <thead>
              <tr>
                <th>Order</th>
                <th>Region</th>
                <th>Month</th>
                <th>Channel</th>
                <th>USD</th>
              </tr>
            </thead>
            <tbody>
              @for (sale of detail(); track sale.id) {
                <tr>
                  <td>{{ sale.id }}</td>
                  <td>{{ sale.region }}</td>
                  <td>{{ sale.month }}</td>
                  <td>{{ sale.channel }}</td>
                  <td>{{ sale.amount }}</td>
                </tr>
              } @empty {
                <tr>
                  <td colspan="5">No orders in this cell.</td>
                </tr>
              }
            </tbody>
          </table>
        </div>
      </section>
    }
  </section>`,
})
export class AnalyticsExamples {
  readonly channel = signal("");
  readonly dimension = signal<"region" | "channel">("region");
  readonly chartType = signal<"bar" | "line">("bar");
  readonly order = signal<string[]>([]);
  readonly drill = signal<{ row: string; column: string } | null>(null);
  readonly filtered = computed(() =>
    sales.filter((sale) => !this.channel() || sale.channel === this.channel()),
  );
  readonly rowSelector = computed(() => {
    const key = this.dimension();
    return (sale: Sale) => sale[key];
  });
  readonly column = (sale: Sale) => sale.month;
  readonly measure = (sale: Sale) => sale.amount;
  readonly pivot = computed(() =>
    atlasPivot(this.filtered(), this.rowSelector(), this.column, this.measure),
  );
  readonly points = computed(() =>
    this.pivot().rows.map((row) => ({
      id: row.label,
      label: row.label,
      value: Number(row.total),
    })),
  );
  readonly detail = computed(() =>
    this.filtered().filter(
      (sale) =>
        this.rowSelector()(sale) === this.drill()?.row &&
        (!this.drill()?.column || sale.month === this.drill()?.column),
    ),
  );
}
let planningSequence = 0;
@Component({
  selector: "demo-planning",
  imports: [AtlasButton, AtlasInput, AtlasCalendar, AtlasRichText, FormField],
  styleUrl: "./advanced-examples.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: ` <section class="advanced-demo">
    <header>
      <div>
        <h3>Delivery planning & notes</h3>
        <p>
          Browse dates, select a delivery and edit its structured instructions.
        </p>
      </div>
      <button
        atlasButton
        (click)="workspace.open({ screen: 'bqatlas.planning', data: {} })"
      >
        Open independent plan
      </button>
    </header>
    <div class="advanced-tools">
      <button
        atlasButton
        variant="primary"
        [disabled]="store.saving() || store.readonly() || !store.dirty()"
        (click)="save()"
      >
        {{ store.saving() ? "Saving…" : "Save plan" }}</button
      ><button
        atlasButton
        [disabled]="!store.dirty() && !store.saving()"
        (click)="store.reset()"
      >
        Reset draft</button
      ><label
        ><input
          type="checkbox"
          [checked]="store.readonly()"
          (change)="store.readonly.set($any($event.target).checked)"
        />
        Read-only</label
      ><label
        ><input
          type="checkbox"
          [checked]="store.failNext()"
          (change)="store.failNext.set($any($event.target).checked)"
        />
        Fail next save</label
      ><span>{{ store.dirty() ? "Unsaved changes" : "Locally saved" }}</span>
    </div>
    <div class="advanced-columns">
      <div class="advanced-card">
        <atlas-calendar
          [(month)]="month"
          [(selectedDate)]="selected"
          [events]="events()"
          label="Delivery calendar"
          (eventSelected)="eventMessage.set($event.title)"
        />
        <p role="status">{{ eventMessage() }}</p>
        <button
          atlasButton
          [disabled]="store.readonly() || store.saving()"
          (click)="store.update({ date: selected() })"
        >
          Schedule on {{ selected() }}
        </button>
      </div>
      <div class="advanced-card">
        <label
          >Delivery title<input
            atlasInput
            [value]="store.draft().title"
            [disabled]="store.saving()"
            [readOnly]="store.readonly()"
            (input)="
              store.update({ title: $any($event.target).value })
            " /></label
        ><label
          >Delivery date<input
            atlasInput
            type="date"
            [value]="store.draft().date"
            [disabled]="store.saving()"
            [readOnly]="store.readonly()"
            (input)="
              store.update({ date: $any($event.target).value })
            " /></label
        ><atlas-rich-text
          [controlId]="id"
          label="Dispatch notes"
          [formField]="fields.notes"
        />
      </div>
    </div>
    @if (store.error()) {
      <p role="alert" class="atlas-edit-alert">{{ store.error() }}</p>
    }
    <p role="status">{{ store.message() }}</p>
    <p class="atlas-muted">
      Local demonstration. Notes contain headings, paragraphs, bullets and
      whole-block emphasis; saving writes no backend records.
    </p>
  </section>`,
})
export class PlanningExamples {
  readonly store = new PlanningStore();
  readonly workspace = inject(WorkspaceService);
  readonly id = `dispatch-notes-${++planningSequence}`;
  readonly month = signal("2026-09");
  readonly selected = signal("2026-09-28");
  readonly eventMessage = signal("");
  readonly fields = form(this.store.draft, (schema) => {
    disabled(schema, { when: () => this.store.saving() });
    readonly(schema, { when: () => this.store.readonly() });
  });
  readonly events = computed(() => [
    {
      id: "dispatch",
      title: this.store.draft().title,
      date: this.store.draft().date,
    },
    {
      id: "stocktake",
      title: "Warehouse stocktake",
      date: "2026-09-24",
      endDate: "2026-09-25",
    },
    { id: "inbound", title: "Inbound shipment", date: "2026-09-28" },
  ]);
  constructor() {
    inject(DestroyRef).onDestroy(() => this.store.dispose());
  }
  async save() {
    try {
      await this.store.save();
    } catch {
      /* Error remains visible in the draft. */
    }
  }
}
@Component({
  selector: "demo-planning-task",
  imports: [PlanningExamples],
  template: `<div style="padding:18px;min-width:0"><demo-planning /></div>`,
})
export class PlanningDemoTask {
  readonly demo = viewChild(PlanningExamples);
  readonly task = inject(ATLAS_TASK);
  constructor() {
    this.task.lifecycle.save = async () => {
      await this.demo()?.store.save();
    };
    effect(() => this.task.dirty.set(this.demo()?.store.dirty() ?? false));
  }
}
interface Inventory {
  id: string;
  name: string;
  warehouse: string;
  quantity: number;
  price: string;
}
@Component({
  selector: "demo-virtual-inventory",
  imports: [AtlasButton, AtlasInput, AtlasVirtualGrid],
  styleUrl: "./advanced-examples.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: ` <section class="advanced-demo">
    <header>
      <div>
        <h3>Virtualized inventory</h3>
        <p>
          50,000 loaded sample records. Only the visible rows and a small buffer
          are mounted.
        </p>
      </div>
    </header>
    <div class="advanced-tools">
      <label
        >Find inventory<input
          atlasInput
          type="search"
          [value]="search()"
          (input)="search.set($any($event.target).value); selected.set(null)"
          placeholder="SKU or product name" /></label
      ><label
        >Warehouse<select
          atlasInput
          [value]="warehouse()"
          (change)="
            warehouse.set($any($event.target).value); selected.set(null)
          "
        >
          <option value="">All warehouses</option>
          <option>Dhaka</option>
          <option>Chattogram</option>
          <option>Sylhet</option>
        </select></label
      ><label
        >Sort<select
          atlasInput
          [value]="sort()"
          (change)="sort.set($any($event.target).value)"
        >
          <option value="id">SKU</option>
          <option value="quantity">Quantity descending</option>
        </select></label
      ><label
        >Row density<select
          atlasInput
          [value]="rowHeight()"
          (change)="rowHeight.set(+$any($event.target).value)"
        >
          <option value="32">Compact</option>
          <option value="44">Comfortable</option>
        </select></label
      ><button atlasButton (click)="reload(false)">Reload sample</button
      ><button atlasButton (click)="reload(true)">Simulate load failure</button>
    </div>
    <atlas-virtual-grid
      [rows]="filtered()"
      [columns]="columns"
      keyField="id"
      label="Inventory records"
      [rowHeight]="rowHeight()"
      [height]="400"
      [(selectedKey)]="selected"
      [loading]="loading()"
      [error]="error()"
      (retry)="reload(false)"
      (rowActivated)="opened.set($event)"
    />
    @if (opened(); as row) {
      <section class="advanced-card">
        <header>
          <h4>{{ row.id }} · {{ row.name }}</h4>
          <button atlasButton (click)="opened.set(null)">
            Close record details
          </button>
        </header>
        <p>
          {{ row.warehouse }} · {{ row.quantity }} units · USD {{ row.price }}
        </p>
      </section>
    }
    <p class="atlas-muted">
      Selection: {{ selected() || "none" }}. Double-click or press Enter to
      inspect a row. Filtering and sorting run locally; this example does not
      fetch server pages.
    </p>
  </section>`,
})
export class VirtualInventoryExamples {
  readonly records: Inventory[] = Array.from({ length: 50000 }, (_, index) => ({
    id: `SKU-${String(index + 1).padStart(5, "0")}`,
    name: `Industrial part ${index + 1}`,
    warehouse: ["Dhaka", "Chattogram", "Sylhet"][index % 3],
    quantity: (index * 7919) % 1000,
    price: ((10000 + (index % 10000)) / 100).toFixed(2),
  }));
  readonly search = signal("");
  readonly warehouse = signal("");
  readonly sort = signal("id");
  readonly rowHeight = signal(32);
  readonly selected = signal<string | null>(null);
  readonly opened = signal<Inventory | null>(null);
  readonly loading = signal(false);
  readonly error = signal("");
  private generation = 0;
  readonly columns: AtlasColumn<Inventory>[] = [
    { key: "id", label: "SKU", width: 150 },
    { key: "name", label: "Product", width: 260 },
    { key: "warehouse", label: "Warehouse", width: 160 },
    { key: "quantity", label: "On hand", width: 110, align: "right" },
    { key: "price", label: "Price · USD", width: 130, align: "right" },
  ];
  readonly filtered = computed(() => {
    const query = this.search().trim().toLowerCase();
    const rows = this.records.filter(
      (row) =>
        (!this.warehouse() || row.warehouse === this.warehouse()) &&
        (!query || `${row.id} ${row.name}`.toLowerCase().includes(query)),
    );
    return this.sort() === "quantity"
      ? rows.sort((a, b) => b.quantity - a.quantity || a.id.localeCompare(b.id))
      : rows;
  });
  constructor() {
    inject(DestroyRef).onDestroy(() => {
      this.generation++;
    });
  }
  async reload(fail: boolean) {
    const generation = ++this.generation;
    this.loading.set(true);
    this.error.set("");
    await new Promise((resolve) => setTimeout(resolve, 400));
    if (generation !== this.generation) return;
    this.loading.set(false);
    if (fail)
      this.error.set(
        "Simulated inventory load failure. Retry to restore the local records.",
      );
  }
}
