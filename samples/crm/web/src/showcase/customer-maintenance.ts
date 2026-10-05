import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  computed,
  effect,
  inject,
  input,
  signal,
  untracked,
} from "@angular/core";
import {
  AtlasTab,
  AtlasTabs,
  AtlasSplitPane,
  AtlasFieldset,
  AtlasAccordionSection,
  AtlasPanel,
  AtlasInput,
  AtlasButton,
  AtlasTree,
  AtlasCommandToolbar,
  AtlasDialog,
} from "@bqatlas/ui";
import { CustomerDemoStore, type CustomerIssue } from "./customer-demo-store";
let demoSequence = 0;
@Component({
  selector: "demo-customer-maintenance",
  imports: [
    AtlasTab,
    AtlasTabs,
    AtlasSplitPane,
    AtlasFieldset,
    AtlasAccordionSection,
    AtlasPanel,
    AtlasInput,
    AtlasButton,
    AtlasTree,
    AtlasCommandToolbar,
    AtlasDialog,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `<div class="customer-demo">
    <div class="lab-section-intro">
      <div>
        <h3>Customer maintenance</h3>
        <p>
          A complete local workspace: choose a customer, edit across tabs, and
          save all retained drafts.
        </p>
      </div>
      <span class="showcase-badge">Local sample data</span>
    </div>
    <div class="demo-options">
      <label
        ><input
          type="checkbox"
          [checked]="store().readonly()"
          [disabled]="store().saving()"
          (change)="store().readonly.set($any($event.target).checked)"
        />
        Read-only mode</label
      ><label
        ><input
          type="checkbox"
          [checked]="store().failNext()"
          [disabled]="store().saving()"
          (change)="store().failNext.set($any($event.target).checked)"
        />
        Fail next save</label
      ><button
        atlasButton
        [disabled]="store().saving()"
        (click)="resetOpen.set(true)"
      >
        Reset customer demo
      </button>
    </div>
    <atlas-split-pane
      primaryLabel="Customer browser"
      secondaryLabel="Customer record"
      [ratio]="30"
      [minimum]="22"
      [maximum]="55"
      [breakpoint]="720"
    >
      <div atlasSplitPrimary class="customer-browser">
        <atlas-panel title="Customers">
          <atlas-tree
            label="Customer groups"
            [filterable]="false"
            [nodes]="groups"
            [(selectedId)]="group"
            [expandedIds]="['all']"
          />
          <label class="customer-search"
            >Find a customer<input
              atlasInput
              type="search"
              [value]="query()"
              (input)="query.set($any($event.target).value)"
              placeholder="Name, code or city"
          /></label>
          <nav aria-label="Sample customers" class="customer-records">
            @for (row of rows(); track row.id) {
              <button
                type="button"
                [disabled]="store().saving()"
                [attr.aria-current]="
                  store().selected() === row.id ? 'true' : null
                "
                (click)="store().select(row.id)"
              >
                <strong
                  >{{ row.name || "Unnamed customer" }}
                  @if (store().drafts()[row.id]) {
                    <span aria-label="Unsaved changes">•</span>
                  }</strong
                ><small>{{ row.code }} · {{ row.city }}</small
                ><small>{{ row.active ? "Active" : "Inactive" }}</small>
              </button>
            }
            @if (!rows().length) {
              <p class="lab-note" role="status">
                No customers match. Clear the search or choose another group.
              </p>
            }
          </nav>
          <span panelFooter
            >{{ rows().length }} shown · drafts stay when changing
            selection</span
          >
        </atlas-panel>
      </div>
      <div atlasSplitSecondary>
        <atlas-panel [title]="store().current().name || 'Unnamed customer'">
          <span panelActions class="showcase-badge">{{
            store().current().code
          }}</span>
          <atlas-command-toolbar
            panelToolbar
            label="Customer actions"
            [commands]="commands()"
            (command)="save()"
            >{{
              store().saving()
                ? "Saving…"
                : store().dirty()
                  ? "Unsaved customer drafts"
                  : "All changes saved locally"
            }}</atlas-command-toolbar
          >
          @if (store().error()) {
            <p role="alert" class="atlas-edit-alert">{{ store().error() }}</p>
          }
          @if (store().message()) {
            <p role="status" class="atlas-edit-status">
              {{ store().message() }}
            </p>
          }
          @if (store().issues().length) {
            <div class="customer-issues" role="alert">
              <strong>Review customer details</strong>
              @for (issue of store().issues(); track issue.id + issue.field) {
                <button type="button" (click)="focusIssue(issue)">
                  {{ issue.id }} · {{ issue.message }}
                </button>
              }
            </div>
          }
          <atlas-tabs label="Customer record sections" [(selected)]="tab">
            <ng-template atlasTab="details" label="Details">
              <atlas-fieldset label="Company details"
                ><div class="form-grid">
                  <label [for]="uid + '-name'"
                    >Customer name *<input
                      atlasInput
                      [id]="uid + '-name'"
                      aria-label="Customer name"
                      required
                      [value]="store().current().name"
                      [disabled]="store().saving()"
                      [readOnly]="store().readonly()"
                      [attr.aria-invalid]="fieldError('name') ? true : null"
                      [attr.aria-describedby]="
                        fieldError('name') ? uid + '-name-error' : null
                      "
                      (input)="
                        store().change('name', $any($event.target).value)
                      "
                    />
                    @if (fieldError("name")) {
                      <small
                        class="atlas-field-error"
                        [id]="uid + '-name-error'"
                        >{{ fieldError("name") }}</small
                      >
                    }
                  </label>
                  <label
                    >City<input
                      atlasInput
                      [value]="store().current().city"
                      [disabled]="store().saving()"
                      [readOnly]="store().readonly()"
                      (input)="
                        store().change('city', $any($event.target).value)
                      "
                  /></label>
                  <label
                    >Payment terms<select
                      atlasInput
                      [value]="store().current().terms"
                      [disabled]="store().saving() || store().readonly()"
                      (change)="
                        store().change('terms', $any($event.target).value)
                      "
                    >
                      <option>Net 30</option>
                      <option>Net 60</option>
                      <option>Due on receipt</option>
                    </select></label
                  >
                  <label class="customer-active"
                    ><input
                      type="checkbox"
                      [checked]="store().current().active"
                      [disabled]="store().saving() || store().readonly()"
                      (change)="
                        store().change('active', $any($event.target).checked)
                      "
                    />
                    Active account</label
                  >
                </div></atlas-fieldset
              >
              <atlas-accordion-section label="Delivery instructions"
                ><label class="customer-notes"
                  >Internal notes<textarea
                    atlasInput
                    rows="3"
                    [value]="store().current().notes"
                    [disabled]="store().saving()"
                    [readOnly]="store().readonly()"
                    (input)="store().change('notes', $any($event.target).value)"
                  ></textarea></label
              ></atlas-accordion-section>
            </ng-template>
            <ng-template atlasTab="contacts" label="Contacts">
              <atlas-fieldset label="Primary contact"
                ><div class="form-grid">
                  <label [for]="uid + '-email'"
                    >Contact email *<input
                      atlasInput
                      [id]="uid + '-email'"
                      aria-label="Contact email"
                      type="email"
                      required
                      [value]="store().current().email"
                      [disabled]="store().saving()"
                      [readOnly]="store().readonly()"
                      [attr.aria-invalid]="fieldError('email') ? true : null"
                      [attr.aria-describedby]="
                        fieldError('email') ? uid + '-email-error' : null
                      "
                      (input)="
                        store().change('email', $any($event.target).value)
                      "
                    />
                    @if (fieldError("email")) {
                      <small
                        class="atlas-field-error"
                        [id]="uid + '-email-error'"
                        >{{ fieldError("email") }}</small
                      >
                    }
                  </label>
                  <label
                    >Contact phone<input
                      atlasInput
                      type="tel"
                      [value]="store().current().phone"
                      [disabled]="store().saving()"
                      [readOnly]="store().readonly()"
                      (input)="
                        store().change('phone', $any($event.target).value)
                      "
                  /></label></div
              ></atlas-fieldset>
              <p class="lab-note">
                Contact edits are part of the same customer draft as company
                details.
              </p>
            </ng-template>
            <ng-template atlasTab="history" label="History"
              ><h4>Local activity</h4>
              <ul class="customer-history">
                @for (
                  entry of store().history()[store().selected()] ?? [];
                  track $index
                ) {
                  <li>{{ entry }}</li>
                } @empty {
                  <li>No saves in this demo session.</li>
                }
              </ul>
              <p class="lab-note">
                This activity is illustrative and is not a server audit log.
              </p></ng-template
            >
          </atlas-tabs>
          <span panelFooter
            >{{ store().current().id }} ·
            {{
              store().readonly()
                ? "Read-only preview"
                : "Edits are kept in this example window"
            }}
            · No API requests</span
          >
        </atlas-panel>
      </div>
    </atlas-split-pane>
    <atlas-dialog title="Reset customer demo?" [(open)]="resetOpen"
      ><p>
        Discard all customer drafts and restore the sample customers. Other
        examples are unchanged.
      </p>
      <div atlasDialogActions>
        <button atlasButton (click)="resetOpen.set(false)">Keep editing</button
        ><button
          atlasButton
          variant="danger"
          (click)="store().reset(); resetOpen.set(false)"
        >
          Reset demo
        </button>
      </div></atlas-dialog
    >
  </div>`,
})
export class CustomerMaintenance {
  readonly uid = `customer-demo-${++demoSequence}`;
  readonly store = input.required<CustomerDemoStore>();
  readonly tab = signal("details");
  readonly group = signal<string | null>("all");
  readonly query = signal("");
  readonly resetOpen = signal(false);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  readonly groups = [
    {
      id: "all",
      label: "All customers",
      children: [
        { id: "active", label: "Active accounts" },
        { id: "inactive", label: "Inactive accounts" },
      ],
    },
  ];
  readonly rows = computed(() =>
    this.store()
      .saved()
      .map((row) => this.store().drafts()[row.id] ?? row)
      .filter(
        (row) =>
          (this.group() === "all" ||
            this.group() === null ||
            (this.group() === "active" ? row.active : !row.active)) &&
          `${row.name} ${row.code} ${row.city}`
            .toLowerCase()
            .includes(this.query().toLowerCase().trim()),
      ),
  );
  readonly commands = computed(() => [
    {
      id: "save",
      label: "Save customer drafts",
      icon: "save",
      primary: true,
      disabled:
        !this.store().dirty() ||
        this.store().saving() ||
        this.store().readonly(),
    },
  ]);
  constructor() {
    effect(() => {
      if (this.store().focusRequest())
        untracked(() => {
          const issue = this.store().issues()[0];
          if (issue) this.focusIssue(issue);
        });
    });
  }
  fieldError(field: string) {
    return this.store()
      .issues()
      .find(
        (issue) =>
          issue.id === this.store().selected() && issue.field === field,
      )?.message;
  }
  focusIssue(issue: CustomerIssue) {
    this.store().select(issue.id);
    this.tab.set(issue.tab);
    setTimeout(
      () =>
        this.host.nativeElement
          .querySelector<HTMLInputElement>(`#${this.uid}-${issue.field}`)
          ?.focus(),
      0,
    );
  }
  async save() {
    try {
      await this.store().save();
    } catch {
      /* Store renders validation and failure state. */
    }
  }
}
