import { FeedbackExamples } from "./feedback-examples";
import {
  AnalyticsExamples,
  PlanningExamples,
  VirtualInventoryExamples,
} from "./advanced-examples";
import { CommandExamples } from "./command-examples";
import { GroupedExamples } from "./grouped-examples";
import { HierarchyExamples } from "./hierarchy-examples";
import {
  ChangeDetectionStrategy,
  Component,
  ViewEncapsulation,
  computed,
  effect,
  inject,
  signal,
  viewChild,
} from "@angular/core";
import { FormsModule } from "@angular/forms";
import { ATLAS_TASK, AtlasButton, AtlasInput } from "@bqatlas/ui";
import { CrudWorkspace } from "@bqatlas/angular";
import { EditingExamples } from "./editing-examples";
import { PurchaseOrderDemo } from "./purchase-order-demo";
import { SelectionEntryExamples } from "./selection-entry-examples";
import { ErpControlExamples } from "./erp-control-examples";
import { CollectionExamples } from "./collection-examples";
import { CustomerMaintenance } from "./customer-maintenance";
import { CustomerDemoStore } from "./customer-demo-store";
import { LayoutExamples } from "./layout-examples";
import { BusinessExamples } from "./business-examples";
@Component({
  selector: "app-showcase",
  imports: [
    FormsModule,
    AtlasButton,
    AtlasInput,
    EditingExamples,
    SelectionEntryExamples,
    ErpControlExamples,
    CollectionExamples,
    BusinessExamples,
    CustomerMaintenance,
    LayoutExamples,
    GroupedExamples,
    HierarchyExamples,
    CommandExamples,
    FeedbackExamples,
    AnalyticsExamples,
    PlanningExamples,
    VirtualInventoryExamples,
  ],
  providers: [PurchaseOrderDemo],
  changeDetection: ChangeDetectionStrategy.OnPush,
  encapsulation: ViewEncapsulation.None,
  styleUrl: "./showcase.css",
  template: `<section class="showcase" [attr.data-density]="density()">
    <header class="showcase-heading">
      <div>
        <span class="showcase-eyebrow">ATLAS EXAMPLES</span>
        <h2>Forms & controls</h2>
        <p>Explore real controls in everyday business workflows.</p>
      </div>
      <span class="showcase-badge">Sample workspace</span>
    </header>
    <div class="example-browser" [class.preview-narrow]="narrowPreview()">
      <aside class="example-sidebar">
        <label class="example-search"
          >Find an example<input
            atlasInput
            type="search"
            placeholder="Search controls or workflows…"
            [value]="search()"
            (input)="search.set($any($event.target).value)"
        /></label>
        <label class="example-mobile-picker"
          >Choose example<select
            atlasInput
            [value]="filteredSelection()"
            (change)="select($any($event.target).value)"
          >
            <option value="" disabled>Choose a matching example…</option>
            @for (item of filtered(); track item.id) {
              <option [value]="item.id">{{ item.title }}</option>
            }
          </select></label
        >
        <nav aria-label="Example categories">
          @for (group of categories; track group) {
            @if (matches(group).length) {
              <h3>{{ group }}</h3>
            }
            @for (item of matches(group); track item.id) {
              <button
                type="button"
                [attr.aria-current]="section() === item.id ? 'page' : null"
                (click)="select(item.id)"
              >
                <strong>{{ item.title }}</strong
                ><small>{{
                  item.id === "crud"
                    ? "Live application"
                    : item.id === "overview"
                      ? "Example guide"
                      : "Local demonstration"
                }}</small>
              </button>
            }
          }
        </nav>
        @if (!filtered().length) {
          <p role="status">No matching examples.</p>
          <button atlasButton (click)="search.set('')">Clear search</button>
        }
      </aside>
      <div class="example-main">
        <div class="example-tools">
          <label
            >Density<select
              atlasInput
              [value]="density()"
              (change)="density.set($any($event.target).value)"
            >
              <option value="compact">Compact</option>
              <option value="comfortable">Comfortable</option>
            </select></label
          ><label
            ><input
              type="checkbox"
              [checked]="narrowPreview()"
              (change)="narrowPreview.set($any($event.target).checked)"
            />
            Narrow preview</label
          >
        </div>
        <p class="showcase-notice">
          Local demos reset when this window closes. CRUD links open real
          records. Switching examples preserves your work.
        </p>
        <div class="showcase-content">
          <section
            [hidden]="section() !== 'overview'"
            aria-label="Example guide"
          >
            <div class="showcase-cards">
              @for (item of sections.slice(1); track item.id) {
                <button class="showcase-card" (click)="select(item.id)">
                  <strong>{{ item.title }}</strong
                  ><span>{{ item.description }}</span
                  ><small>Explore example →</small>
                </button>
              }
            </div>
          </section>
          <section [hidden]="section() !== 'crud'" aria-label="CRUD examples">
            <div class="lab-section-intro">
              <div>
                <h3>From a simple record to a full aggregate</h3>
                <p>
                  These views use the actual backend, permissions, validation
                  and optimistic concurrency.
                </p>
              </div>
            </div>
            <div class="showcase-cards">
              @for (item of crud.menus(); track item.resource) {
                <button
                  class="showcase-card"
                  (click)="crud.openList(item.resource)"
                >
                  <strong>{{ item.title }}</strong
                  ><span>{{
                    item.resource === "sales.quotes"
                      ? "Header and child lines, remote customer lookup, exact decimal totals and submit workflow."
                      : "Metadata-driven CRUD: create, edit, filter, validate and delete records."
                  }}</span
                  ><small>Open real records →</small>
                </button>
              }
            </div>
            @if (!crud.menus().length) {
              <p>No CRUD views are available for your account.</p>
            }
          </section>
          @if (visited().has("master")) {
            <demo-editing [hidden]="section() !== 'master'" />
          }
          @if (visited().has("inputs")) {
            <demo-selection-entry [hidden]="section() !== 'inputs'" />
          }
          @if (visited().has("lookup")) {
            <demo-erp-controls [hidden]="section() !== 'lookup'" />
          }
          @if (visited().has("tables")) {
            <demo-collections [hidden]="section() !== 'tables'" />
          }
          @if (visited().has("business")) {
            <demo-business [hidden]="section() !== 'business'" />
          }
          @if (visited().has("customer")) {
            <demo-customer-maintenance
              [hidden]="section() !== 'customer'"
              [store]="customers"
            />
          }
          @if (visited().has("layouts")) {
            <demo-layouts [hidden]="section() !== 'layouts'" />
          }
          @if (visited().has("grouped")) {
            <demo-grouped-tables [hidden]="section() !== 'grouped'" />
          }
          @if (visited().has("hierarchy")) {
            <demo-hierarchies [hidden]="section() !== 'hierarchy'" />
          }
          @if (visited().has("commands")) {
            <demo-commands [hidden]="section() !== 'commands'" />
          }
          @if (visited().has("feedback")) {
            <demo-feedback [hidden]="section() !== 'feedback'" />
          }
          @if (visited().has("analytics")) {
            <demo-analytics [hidden]="section() !== 'analytics'" />
          }
          @if (visited().has("planning")) {
            <demo-planning [hidden]="section() !== 'planning'" />
          }
          @if (visited().has("virtual")) {
            <demo-virtual-inventory [hidden]="section() !== 'virtual'" />
          }
          @if (section() !== "overview") {
            <details class="example-guide">
              <summary>About this example · API & keyboard</summary>
              <p>{{ activeExample()?.description }}</p>
              <dl>
                <dt>Components</dt>
                <dd>
                  {{
                    apiNotes[section()] ||
                      "Existing Atlas form and collection controls"
                  }}
                </dd>
                <dt>Keyboard</dt>
                <dd>
                  {{
                    section() === "customer" || section() === "layouts"
                      ? "Tabs: Left/Right, Home/End. Splitter: Left/Right, Shift for larger steps, Home/End for limits. Enter/Space toggles a fieldset or accordion."
                      : "Tab to controls; arrows navigate choices; Escape closes popups. See the controls guide for detailed shortcuts."
                  }}
                </dd>
                <dt>Data</dt>
                <dd>
                  {{
                    section() === "crud"
                      ? "Uses application APIs and current permissions."
                      : "Local demo values. No business records are created."
                  }}
                </dd>
              </dl>
            </details>
          }
        </div>
      </div>
    </div>
  </section>`,
})
export class Showcase {
  readonly planningDemo = viewChild(PlanningExamples);
  readonly commandsDemo = viewChild(CommandExamples);
  readonly crud = inject(CrudWorkspace);
  readonly store = inject(PurchaseOrderDemo);
  readonly task = inject(ATLAS_TASK);
  readonly section = signal("overview");
  readonly search = signal("");
  readonly density = signal("compact");
  readonly narrowPreview = signal(false);
  readonly visited = signal(new Set(["overview"]));
  readonly customers = new CustomerDemoStore();
  readonly categories = [
    "Start here",
    "Business screens",
    "Layout & forms",
    "Data & selection",
  ];
  readonly filtered = computed(() =>
    this.sections.filter((item) =>
      `${item.title} ${item.description} ${this.category(item.id)}`
        .toLowerCase()
        .includes(this.search().trim().toLowerCase()),
    ),
  );
  readonly filteredSelection = computed(() =>
    this.filtered().some((item) => item.id === this.section())
      ? this.section()
      : "",
  );
  readonly activeExample = computed(() =>
    this.sections.find((item) => item.id === this.section()),
  );
  readonly apiNotes: Record<string, string> = {
    feedback:
      "AtlasMessageBox, AtlasNotice, AtlasProgress, AtlasLoadingRegion, AtlasTooltip, AtlasPopover and AtlasStatusBar. Escape dismisses dialogs/help; Tab navigates actions. Cancellation retains the local preview.",
    analytics:
      "AtlasPivotTable, AtlasChart, AtlasDashboard / AtlasDashboardPanel. Panel buttons support keyboard reordering; chart points support Enter/Space and include a data table.",
    planning:
      "AtlasCalendar: arrows, Home/End, PageUp/PageDown. AtlasRichText integrates with signal forms and stores structured blocks.",
    virtual:
      "AtlasVirtualGrid: fixed-height loaded rows, external filters/sort, arrows/PageUp/PageDown/Home/End and Enter to activate.",
    commands:
      "AtlasPopupMenu, AtlasContextMenu, AtlasMenuButton, AtlasSplitButton, AtlasCommandToolbar and AtlasItemSelector. Menus: Up/Down, Home/End, initial letter, Escape. Selector: Alt+arrows; Ctrl/⌘ or Shift for multiple selection.",
    customer:
      "AtlasSplitPane, AtlasPanel, AtlasTree, AtlasTabs / AtlasTab, AtlasFieldset, AtlasAccordionSection, AtlasCommandToolbar",
    layouts:
      "AtlasSplitPane: ratio / minimum / maximum / collapsed. AtlasTabs: selected. AtlasFieldset: collapsed / disabled. Panel slots: panelActions / panelToolbar / panelFooter.",
    master:
      "AtlasMasterDetail, AtlasEditableGrid, AtlasCommandToolbar, AtlasDialog",
    tables: "AtlasTable, AtlasTree, AtlasListView",
    grouped:
      "AtlasTable: groupBy, collapsedGroups, summaries, serverSummary; AtlasColumn.group; atlasSumDecimal and atlasSummaryQueryKey.",
    hierarchy:
      "AtlasCheckTree: checkedIds, expandedIds; AtlasTreeGrid: columns, selectedId, expandedIds, loadChildren. Arrows/Home/End navigate; Space selects/checks; Enter retries a failed load.",
    lookup: "AtlasLookup with typed columns and remote query state",
    inputs:
      "AtlasSelect, AtlasMultiSelect, AtlasAutocomplete, AtlasRadioGroup, AtlasCheckboxGroup, AtlasDateInput, AtlasDecimalInput",
  };
  category(id: string) {
    return id === "overview"
      ? "Start here"
      : ["customer", "master", "crud", "business"].includes(id)
        ? "Business screens"
        : ["layouts", "feedback"].includes(id)
          ? "Layout & forms"
          : "Data & selection";
  }
  matches(category: string) {
    return this.filtered().filter(
      (item) => this.category(item.id) === category,
    );
  }
  select(id: string) {
    this.visited.update((seen) => new Set([...seen, id]));
    this.section.set(id);
  }

  readonly sections = [
    { id: "overview", title: "Overview", description: "" },
    {
      id: "feedback",
      title: "Feedback & guidance",
      description:
        "Alerts, confirmations, prompts, contextual help and a cancellable import with loading, progress, retry and status.",
    },
    {
      id: "analytics",
      title: "Analytics & dashboards",
      description:
        "Exact decimal pivots, bar and line charts, order drill-down and rearrangeable panels.",
    },
    {
      id: "planning",
      title: "Planning & notes",
      description:
        "Delivery calendar, structured rich-text notes, signal forms and retained local drafts.",
    },
    {
      id: "virtual",
      title: "Virtualized inventory",
      description:
        "50,000 records, bounded rendering, keyboard navigation and local filtering/sorting.",
    },
    {
      id: "commands",
      title: "Commands & assignment",
      description:
        "Permission-aware menus, split buttons, responsive toolbar overflow and warehouse item selection with retained price-list drafts.",
    },
    {
      id: "grouped",
      title: "Grouped order register",
      description:
        "Collapsible groups, grouped headers, exact page and filtered totals; simulated server aggregates and retry.",
    },
    {
      id: "hierarchy",
      title: "Accounts & assemblies",
      description:
        "Tri-state check tree, chart-of-accounts tree grid and lazily loaded bill of materials.",
    },
    {
      id: "customer",
      title: "Customer maintenance",
      description:
        "Tree navigation, resizable panes, tabbed details and contacts, retained drafts and linked validation.",
    },
    {
      id: "master",
      title: "Master / child",
      description:
        "Purchase orders with editable lines, totals, local save and failed-save recovery.",
    },
    {
      id: "crud",
      title: "CRUD forms",
      description:
        "Open the real customer CRUD and Sales quote aggregate screens.",
    },
    {
      id: "layouts",
      title: "Layout workbench",
      description:
        "Reusable tabs, collapsible fieldsets, accordions, split panes and consistent panel slots.",
    },
    {
      id: "lookup",
      title: "Multi-column lookup",
      description:
        "Compare code, name, location and balance before choosing a customer.",
    },
    {
      id: "inputs",
      title: "Selection & inputs",
      description:
        "Radio, checkboxes, multi-select, autocomplete, dates and decimal entry.",
    },
    {
      id: "tables",
      title: "Advanced tables",
      description:
        "Column filters, multi-sort, resizing, pinning, selection, row details and paging.",
    },
    {
      id: "business",
      title: "Business panels",
      description:
        "Property sheets, totals, activity, attachments and saved filter views.",
    },
  ];
  constructor() {
    this.task.lifecycle.save = async () => {
      if (this.planningDemo()?.store.dirty()) {
        try {
          await this.planningDemo()!.store.save();
        } catch (error) {
          this.select("planning");
          throw error;
        }
      }

      if (this.commandsDemo()?.store.dirty()) {
        try {
          await this.commandsDemo()!.store.execute("save");
        } catch (error) {
          this.select("commands");
          throw error;
        }
      }
      if (this.customers.dirty()) {
        try {
          await this.customers.save();
        } catch (error) {
          this.select("customer");
          throw error;
        }
      }
      if (this.store.dirty()) {
        try {
          await this.store.save();
        } catch (error) {
          this.select("master");
          throw error;
        }
      }
    };
    effect(() =>
      this.task.dirty.set(
        (this.planningDemo()?.store.dirty() ?? false) ||
          this.store.dirty() ||
          this.customers.dirty() ||
          (this.commandsDemo()?.store.dirty() ?? false),
      ),
    );
  }
}
