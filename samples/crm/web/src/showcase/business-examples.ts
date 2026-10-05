import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  computed,
  inject,
  signal,
} from "@angular/core";
import {
  AtlasPropertySheet,
  AtlasProperty,
  AtlasPropertyValue,
  AtlasTotalsPanel,
  AtlasTotalLine,
  AtlasActivityTimeline,
  AtlasActivity,
  AtlasAttachmentList,
  AtlasAttachment,
  AtlasFilterBuilder,
  AtlasFilterField,
  AtlasFilterRule,
  AtlasSavedView,
  matchesFilterRules,
  AtlasButton,
  AtlasInput,
  AtlasDialog,
} from "@bqatlas/ui";
@Component({
  selector: "demo-business",
  imports: [
    AtlasPropertySheet,
    AtlasTotalsPanel,
    AtlasActivityTimeline,
    AtlasAttachmentList,
    AtlasFilterBuilder,
    AtlasButton,
    AtlasInput,
    AtlasDialog,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `<div class="lab-section-intro">
      <div>
        <h3>Business record workbench</h3>
        <p>
          Reusable panels for a customer record. Changes are local demo state;
          no files are uploaded.
        </p>
      </div>
    </div>
    <div class="business-demo-grid">
      <section class="business-demo-card">
        <h3>Property sheet</h3>
        <atlas-property-sheet [fields]="fields" [(value)]="properties" /><button
          atlasButton
          (click)="resetProperties()"
        >
          Reset properties
        </button>
        <p class="atlas-muted">
          Live editing. Adjustments below use these values.
        </p>
      </section>
      <section class="business-demo-card">
        <h3>Totals and adjustments</h3>
        <p>
          Base order: $1,000. Discount applies before tax; shipping is added
          last.
        </p>
        @if (adjustmentError()) {
          <p role="alert" class="atlas-edit-alert">{{ adjustmentError() }}</p>
        } @else {
          <atlas-totals-panel [lines]="totals()" />
        }
        <p class="atlas-muted">
          Calculations belong to the application. This demo uses USD and
          excludes shipping from tax.
        </p>
      </section>
      <section class="business-demo-card">
        <h3>Activity timeline</h3>
        <label
          >Add an internal note<textarea
            atlasInput
            [value]="note()"
            (input)="note.set($any($event.target).value)"
            maxlength="500"
          ></textarea></label
        ><button atlasButton [disabled]="!note().trim()" (click)="addNote()">
          Add note</button
        ><atlas-activity-timeline [events]="activity()" />
      </section>
      <section class="business-demo-card">
        <h3>Document attachments</h3>
        <p>
          Choose a local file (maximum 2 MB) or add a sample. Upload progress is
          simulated.
        </p>
        <label
          ><input
            type="checkbox"
            [checked]="failNext()"
            (change)="failNext.set($any($event.target).checked)"
          />
          Fail next upload</label
        ><button atlasButton (click)="addSample()">Add sample document</button>
        @if (fileError()) {
          <p role="alert">{{ fileError() }}</p>
        }
        <atlas-attachment-list
          [files]="attachments()"
          (add)="addFiles($event)"
          (retry)="upload($event, false)"
          (remove)="removeFile($event)"
          (preview)="previewFile($event)"
        />
      </section>
    </div>
    <section class="business-demo-card">
      <h3>Saved views and filter builder</h3>
      <atlas-filter-builder
        [fields]="filterFields"
        [(rules)]="rules"
        [views]="views()"
        (saveRequested)="saveView($event)"
        (deleteRequested)="deleteView($event)"
      />
      <p role="status">
        {{ viewMessage() }} {{ filtered().length }} matching accounts
      </p>
      <div class="business-table-scroll">
        <table class="atlas-datasheet">
          <thead>
            <tr>
              <th>Account</th>
              <th>Region</th>
              <th>Status</th>
              <th>Balance (USD)</th>
            </tr>
          </thead>
          <tbody>
            @for (row of filtered(); track row.id) {
              <tr>
                <td>{{ row.name }}</td>
                <td>{{ row.region }}</td>
                <td>{{ row.status }}</td>
                <td>{{ row.balance }}</td>
              </tr>
            } @empty {
              <tr>
                <td colspan="4">No accounts match these conditions.</td>
              </tr>
            }
          </tbody>
        </table>
      </div>
    </section>
    <atlas-dialog title="Attachment preview" [(open)]="previewOpen"
      ><p>{{ previewName() }}</p>
      <pre class="business-preview">{{ previewText() }}</pre>
      <p class="atlas-muted">
        Text files are shown as plain text. Other formats show metadata only.
      </p></atlas-dialog
    >`,
})
export class BusinessExamples {
  readonly fields: AtlasProperty[] = [
    {
      key: "code",
      label: "Customer code",
      group: "Account",
      type: "text",
      readonly: true,
    },
    { key: "name", label: "Display name", group: "Account", type: "text" },
    {
      key: "tier",
      label: "Service tier",
      group: "Account",
      type: "select",
      options: [
        { value: "standard", label: "Standard" },
        { value: "priority", label: "Priority" },
      ],
    },
    {
      key: "active",
      label: "Active account",
      group: "Account",
      type: "boolean",
    },
    {
      key: "discount",
      label: "Discount (%)",
      group: "Adjustments",
      type: "number",
      min: 0,
      max: 100,
      step: 0.01,
    },
    {
      key: "tax",
      label: "Tax (%)",
      group: "Adjustments",
      type: "number",
      min: 0,
      max: 100,
      step: 0.01,
    },
    {
      key: "shipping",
      label: "Shipping (USD)",
      group: "Adjustments",
      type: "number",
      min: 0,
      max: 10000,
      step: 0.01,
    },
  ];
  readonly initial = {
    code: "AC-100",
    name: "Northstar Studio",
    tier: "priority",
    active: true,
    discount: 5,
    tax: 10,
    shipping: 25,
  };
  readonly properties = signal<Record<string, AtlasPropertyValue>>({
    ...this.initial,
  });
  readonly adjustmentError = computed(() => {
    const p = this.properties();
    return ["discount", "tax", "shipping"].some(
      (k) =>
        typeof p[k] !== "number" ||
        !Number.isFinite(p[k]) ||
        Number(p[k]) < 0 ||
        Number(p[k]) > (k === "shipping" ? 10000 : 100),
    )
      ? "Enter valid adjustment amounts: percentages 0–100, shipping 0–10,000."
      : "";
  });
  readonly totals = computed<AtlasTotalLine[]>(() => {
    const p = this.properties(),
      discount = (1000 * Number(p["discount"])) / 100,
      tax = ((1000 - discount) * Number(p["tax"])) / 100,
      shipping = Number(p["shipping"]);
    return [
      { id: "sub", label: "Subtotal", value: 1000 },
      { id: "disc", label: "Discount", value: -discount },
      { id: "tax", label: "Tax", value: tax },
      { id: "ship", label: "Shipping", value: shipping },
      {
        id: "total",
        label: "Grand total",
        value: 1000 - discount + tax + shipping,
        emphasis: true,
      },
    ];
  });
  resetProperties() {
    this.properties.set({ ...this.initial });
  }
  readonly note = signal("");
  readonly activity = signal<AtlasActivity[]>([
    {
      id: "seed-1",
      title: "Quote approved",
      detail: "Quote QT-2026-014 approved for the new office.",
      actor: "Alex Morgan",
      timestamp: "2026-09-21T09:30:00Z",
      kind: "Status",
    },
    {
      id: "seed-2",
      title: "Customer called",
      detail: "Confirmed the delivery address.",
      actor: "Jamie Chen",
      timestamp: "2026-09-21T08:15:00Z",
      kind: "Call",
    },
  ]);
  private sequence = 0;
  private timers = new Map<string, ReturnType<typeof setInterval>>();
  private sources = new Map<string, File>();
  private destroyed = false;
  private previewRevision = 0;
  constructor() {
    inject(DestroyRef).onDestroy(() => {
      this.destroyed = true;
      this.previewRevision++;
      this.timers.forEach((timer) => clearInterval(timer));
    });
  }
  addNote() {
    const detail = this.note().trim();
    if (!detail) return;
    this.activity.update((a) => [
      {
        id: `note-${++this.sequence}`,
        title: "Internal note",
        detail,
        actor: "Local developer",
        timestamp: new Date().toISOString(),
        kind: "Note",
      },
      ...a,
    ]);
    this.note.set("");
  }
  readonly attachments = signal<AtlasAttachment[]>([]);
  readonly failNext = signal(false);
  readonly fileError = signal("");
  readonly previewOpen = signal(false);
  readonly previewName = signal("");
  readonly previewText = signal("");
  addSample() {
    this.addFiles([
      new File(
        [
          "Northstar Studio\nDelivery checklist\n- Verify quantities\n- Confirm delivery address\n",
        ],
        "delivery-checklist.txt",
        { type: "text/plain" },
      ),
    ]);
  }
  addFiles(files: File[]) {
    this.fileError.set("");
    for (const file of files) {
      if (file.size > 2 * 1024 * 1024) {
        this.fileError.set(`${file.name}: maximum size is 2 MB.`);
        continue;
      }
      const id = `file-${++this.sequence}`;
      this.sources.set(id, file);
      this.attachments.update((a) => [
        ...a,
        {
          id,
          name: file.name,
          size: file.size,
          status: "uploading",
          progress: 0,
        },
      ]);
      const fail = this.failNext();
      this.failNext.set(false);
      this.upload(id, fail);
    }
  }
  upload(id: string, fail: boolean) {
    if (!this.sources.has(id) || this.timers.has(id)) return;
    this.updateFile(id, { status: "uploading", progress: 0, error: undefined });
    let progress = 0;
    const timer = setInterval(() => {
      progress += 25;
      if (progress >= 100) {
        clearInterval(timer);
        this.timers.delete(id);
        this.updateFile(id, {
          status: fail ? "error" : "ready",
          progress: fail ? 75 : 100,
          error: fail
            ? "Simulated upload failed. Retry to continue."
            : undefined,
        });
      } else this.updateFile(id, { progress });
    }, 250);
    this.timers.set(id, timer);
  }
  updateFile(id: string, patch: Partial<AtlasAttachment>) {
    if (!this.destroyed)
      this.attachments.update((a) =>
        a.map((file) => (file.id === id ? { ...file, ...patch } : file)),
      );
  }
  removeFile(id: string) {
    const timer = this.timers.get(id);
    if (timer) clearInterval(timer);
    this.timers.delete(id);
    this.sources.delete(id);
    this.attachments.update((a) => a.filter((f) => f.id !== id));
    this.previewRevision++;
    this.previewOpen.set(false);
  }
  async previewFile(id: string) {
    const file = this.sources.get(id);
    if (!file) return;
    const revision = ++this.previewRevision;
    this.previewName.set(file.name);
    this.previewText.set("Loading preview…");
    this.previewOpen.set(true);
    try {
      const text =
        file.type.startsWith("text/") || /\.(txt|csv|md)$/i.test(file.name)
          ? (await file.text()).slice(0, 20000)
          : `${file.type || "Unknown file type"} · ${file.size} bytes\nVisual preview is not available for this format.`;
      if (!this.destroyed && revision === this.previewRevision)
        this.previewText.set(text);
    } catch {
      if (!this.destroyed && revision === this.previewRevision)
        this.previewText.set("Unable to read this file.");
    }
  }
  readonly filterFields: AtlasFilterField[] = [
    { key: "name", label: "Account", type: "text" },
    { key: "region", label: "Region", type: "text" },
    { key: "status", label: "Status", type: "text" },
    { key: "balance", label: "Balance", type: "number" },
  ];
  readonly accounts = [
    {
      id: "1",
      name: "Northstar Studio",
      region: "APAC",
      status: "Active",
      balance: 1250,
    },
    {
      id: "2",
      name: "Meridian Labs",
      region: "Europe",
      status: "Active",
      balance: 800,
    },
    {
      id: "3",
      name: "Acme Supply",
      region: "APAC",
      status: "On hold",
      balance: 2400,
    },
    {
      id: "4",
      name: "Forma & Co",
      region: "Europe",
      status: "Active",
      balance: 0,
    },
  ];
  readonly rules = signal<AtlasFilterRule[]>([]);
  readonly views = signal<AtlasSavedView[]>([
    {
      id: "active",
      name: "Active accounts",
      rules: [
        { id: "status", field: "status", operator: "equals", value: "Active" },
      ],
    },
    {
      id: "balance",
      name: "High balance",
      rules: [
        {
          id: "balance-rule",
          field: "balance",
          operator: "gte",
          value: "1000",
        },
      ],
    },
  ]);
  readonly viewMessage = signal("");
  readonly filtered = computed(() =>
    this.accounts.filter((row) => matchesFilterRules(row, this.rules())),
  );
  saveView(name: string) {
    if (this.views().some((v) => v.name.toLowerCase() === name.toLowerCase())) {
      this.viewMessage.set(
        "A view with that name already exists. Choose another name.",
      );
      return;
    }
    this.views.update((v) => [
      ...v,
      {
        id: `view-${++this.sequence}`,
        name,
        rules: structuredClone(this.rules()),
      },
    ]);
    this.viewMessage.set("View saved in this demo session.");
  }
  deleteView(id: string) {
    this.views.update((v) => v.filter((view) => view.id !== id));
    this.viewMessage.set("View removed.");
  }
}
