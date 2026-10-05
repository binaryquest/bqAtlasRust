import {
  afterNextRender,
  ElementRef,
  Injector,
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  computed,
  effect,
  inject,
  signal,
  untracked,
} from "@angular/core";
import { FormsModule } from "@angular/forms";
import {
  ATLAS_TASK,
  AtlasButton,
  AtlasColumn,
  AtlasDatasheet,
  AtlasInput,
  AtlasLatestRequest,
  AtlasTable,
  AtlasTableQuery,
  WorkspaceService,
  WorkspaceTask,
} from "@bqatlas/ui";
import {
  ApiError,
  AtlasApi,
  AtlasDecimalTextInput,
  AtlasSession,
  CrudFeature,
  CrudWorkspace,
  EditorTask,
  RecordDraft,
  ReferenceLookup,
  RestLookupProvider,
  RestResourceProvider,
} from "@bqatlas/angular";
import type { QueryRequest, RecordResult } from "@bqatlas/contracts";
import {
  CustomerOption,
  QuoteInput,
  QuoteLine,
  QuoteRecord,
  QuoteSummary,
  newQuote,
  quoteInput,
  quoteTotals,
  validateQuote,
  appendCatalogProduct,
} from "./quote-model";
const endpoint = "/api/v1/sales/quotes";
@Component({
  selector: "app-quote-list",
  imports: [AtlasButton, AtlasTable],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `<section class="bqatlas-view">
    <header class="bqatlas-toolbar">
      <strong>Sales quotes</strong><span class="bqatlas-spacer"></span>
      @if (session.has("crm.customers.read")) {
        <button atlasButton (click)="crud.openList('crm.customers')">
          Customers ↗
        </button>
      }
      @if (session.has("engagement.products.read")) {
        <button atlasButton (click)="crud.openList('engagement.products')">
          Products ↗
        </button>
      }
      @if (
        session.has("sales.quotes.write") && session.has("crm.customers.lookup")
      ) {
        <button atlasButton variant="primary" (click)="create()">
          New quote
        </button>
      }
      <button atlasButton (click)="reload()">Refresh</button>
    </header>
    <atlas-table
      [rows]="rows()"
      [columns]="columns"
      keyField="id"
      label="quotes"
      [server]="true"
      [total]="total()"
      [pageSize]="25"
      [filterable]="true"
      [columnManage]="true"
      [multiSort]="true"
      [loading]="loading()"
      [error]="error()"
      (queryChange)="query($event)"
      (retry)="reload()"
      (rowActivated)="open($event)"
    />
  </section>`,
})
export class QuoteList {
  readonly task = inject(ATLAS_TASK);
  readonly crud = inject(CrudWorkspace);
  readonly session = inject(AtlasSession);
  private readonly provider = new RestResourceProvider<QuoteSummary>(
    inject(AtlasApi),
    endpoint,
  );
  private readonly latest = new AtlasLatestRequest();
  readonly rows = signal<QuoteSummary[]>([]);
  readonly total = signal(0);
  readonly loading = signal(true);
  readonly error = signal("");
  readonly columns: AtlasColumn<QuoteSummary>[] = [
    { key: "number", label: "Quote", width: 250 },
    { key: "customerName", label: "Customer" },
    { key: "date", label: "Date", width: 115 },
    { key: "currency", label: "Currency", sortable: false, width: 80 },
    {
      key: "status",
      label: "Status",
      width: 110,
      badge: true,
      tone: (row) => (row.status === "submitted" ? "success" : "info"),
    },
    {
      key: "total",
      label: "Amount",
      align: "right",
      sortable: false,
      width: 140,
    },
  ];
  private request: QueryRequest = { page: 0, pageSize: 25, search: "" };
  private timer?: ReturnType<typeof setTimeout>;
  constructor() {
    inject(DestroyRef).onDestroy(() => {
      this.latest.cancel();
      clearTimeout(this.timer);
    });
    effect(() => {
      this.crud.revision();
      untracked(() => this.reload());
    });
  }
  query(value: AtlasTableQuery) {
    this.request = {
      page: value.page,
      pageSize: value.pageSize,
      search: value.search,
      sort: value.sort.map((s) => ({ field: s.key, direction: s.direction })),
    };
    this.latest.cancel();
    clearTimeout(this.timer);
    this.loading.set(true);
    this.timer = setTimeout(() => void this.reload(), 150);
  }
  reload() {
    this.loading.set(true);
    this.error.set("");
    return this.latest.run(
      (signal) => this.provider.query(this.request, signal),
      (page) => {
        this.rows.set(page.items);
        this.total.set(page.total);
        this.loading.set(false);
      },
      (error) => {
        this.error.set(
          error instanceof Error ? error.message : "Unable to load quotes.",
        );
        this.loading.set(false);
      },
    );
  }
  create() {
    this.crud.openEditor("sales.quotes", undefined, this.task.id);
  }
  open(quote: QuoteSummary) {
    this.crud.openEditor("sales.quotes", quote.id, this.task.id);
  }
}
@Component({
  selector: "app-quote-editor",
  imports: [
    FormsModule,
    AtlasButton,
    AtlasInput,
    AtlasDatasheet,
    AtlasDecimalTextInput,
    ReferenceLookup,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    "(keydown.control.s)": "saveShortcut($event)",
    "(keydown.meta.s)": "saveShortcut($event)",
  },
  template: `<section class="bqatlas-view quote-view">
    <header class="bqatlas-toolbar">
      <strong>{{ draft.value().number }}</strong
      ><span
        class="quote-status"
        [class.submitted]="draft.value().status === 'submitted'"
        >{{ draft.value().status }}</span
      ><span class="bqatlas-spacer"></span>
      @if (canEdit()) {
        <button
          atlasButton
          variant="primary"
          [disabled]="loading() || task.saving() || !draft.dirty()"
          (click)="save()"
        >
          Save
        </button>
      }
      @if (canSubmit()) {
        <button
          atlasButton
          [disabled]="loading() || task.saving()"
          (click)="submit()"
        >
          {{ pendingSubmission() ? "Retry submit" : "Submit quote" }}
        </button>
      }
      <button
        atlasButton
        [disabled]="loading() || task.saving()"
        (click)="reload()"
      >
        Reload
      </button>
      @if (draft.capabilities()?.delete) {
        <button
          atlasButton
          variant="danger"
          [disabled]="loading() || task.saving()"
          (click)="remove()"
        >
          Delete
        </button>
      }
    </header>
    @if (task.error()) {
      <div class="atlas-alert error" role="alert">{{ task.error() }}</div>
    }
    @if (loading()) {
      <p role="status">Loading quote…</p>
    }
    <form class="quote-form" (ngSubmit)="save()">
      <fieldset [disabled]="loading() || task.saving()">
        <div class="quote-header-fields">
          <div class="quote-customer">
            <label [for]="task.id + '-customer'">Customer</label>
            <bqatlas-reference-lookup
              [controlId]="task.id + '-customer'"
              label="Customer"
              [describedBy]="task.id + '-customer-error'"
              [provider]="customers"
              resource="crm.customers"
              [readonly]="!canEdit()"
              [columns]="customerColumns"
              [recordKey]="customerKey"
              [displayWith]="customerLabel"
              [selectedText]="selectedCustomer()"
              [value]="draft.value().customerId || null"
              (recordSelected)="chooseCustomer($event)"
              [disabled]="loading() || task.saving()"
              [required]="true"
              [invalid]="!!errors()['customerId']"
            />
            <small
              class="bqatlas-field-error"
              [id]="task.id + '-customer-error'"
              >{{ errors()["customerId"]?.join(" ") }}</small
            >
          </div>
          <div>
            <label [for]="task.id + '-date'">Quote date</label
            ><input
              atlasInput
              type="date"
              [id]="task.id + '-date'"
              [disabled]="!canEdit()"
              name="date"
              [attr.aria-invalid]="!!errors()['date']"
              [attr.aria-describedby]="task.id + '-date-error'"
              [ngModel]="draft.value().date"
              (ngModelChange)="change('date', $event)"
            /><small
              class="bqatlas-field-error"
              [id]="task.id + '-date-error'"
              >{{ errors()["date"]?.join(" ") }}</small
            >
          </div>
          <div>
            <label [for]="task.id + '-currency'">Currency</label
            ><select
              [id]="task.id + '-currency'"
              [disabled]="!canEdit()"
              name="currency"
              [attr.aria-invalid]="!!errors()['currency']"
              [attr.aria-describedby]="task.id + '-currency-error'"
              [ngModel]="draft.value().currency"
              (ngModelChange)="change('currency', $event)"
            >
              <option value="USD">USD</option>
              <option value="EUR">EUR</option>
              <option value="BDT">BDT</option></select
            ><small
              class="bqatlas-field-error"
              [id]="task.id + '-currency-error'"
              >{{ errors()["currency"]?.join(" ") }}</small
            >
          </div>
        </div>
        @if (canEdit() && session.has("engagement.products.read")) {
          <div class="quote-product-picker">
            <label [for]="task.id + '-product'">Add from product catalog</label>
            <bqatlas-reference-lookup
              [controlId]="task.id + '-product'"
              label="Product"
              resource="engagement.products"
              nameField="name"
              [provider]="products"
              [columns]="productColumns"
              [recordKey]="productKey"
              [displayWith]="productLabel"
              [value]="null"
              [contextKey]="draft.value().currency"
              (recordSelected)="addProduct($event)"
              [disabled]="
                loading() || task.saving() || draft.value().lines.length >= 100
              "
            />
            <small
              >Copies the catalog description and price. Choose an active
              product in {{ draft.value().currency }}.</small
            >
          </div>
        }
        <div class="quote-lines-heading">
          <strong>Quote lines</strong><span class="bqatlas-spacer"></span
          ><span>Quantity: 3 decimals · Unit price: 4 decimals</span>
          @if (canEdit()) {
            <button
              atlasButton
              type="button"
              [disabled]="draft.value().lines.length >= 100"
              [attr.aria-invalid]="!!errors()['lines']"
              [attr.aria-describedby]="task.id + '-lines-error'"
              (click)="addLine()"
            >
              Add line
            </button>
          }
        </div>
        <small class="bqatlas-field-error" [id]="task.id + '-lines-error'">{{
          errors()["lines"]?.join(" ")
        }}</small>
        <div class="quote-sheet-scroll">
          <table atlasDatasheet class="quote-sheet" aria-label="Quote lines">
            <thead>
              <tr>
                <th scope="col">Description</th>
                <th scope="col">Quantity</th>
                <th scope="col">Unit price</th>
                <th scope="col">Amount</th>
                <th scope="col">
                  <span class="visually-hidden">Line actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              @for (
                line of draft.value().lines;
                track line.id;
                let i = $index
              ) {
                <tr>
                  <td>
                    <input
                      atlasInput
                      data-sheet-editor
                      [id]="task.id + '-' + line.id + '-description'"
                      [disabled]="!canEdit()"
                      [name]="'description-' + line.id"
                      [attr.aria-label]="'Line ' + (i + 1) + ' description'"
                      [attr.aria-invalid]="!!lineError(i, 'description')"
                      [attr.aria-describedby]="
                        task.id + '-' + line.id + '-description-error'
                      "
                      [ngModel]="line.description"
                      (ngModelChange)="
                        lineChange(line.id, 'description', $event)
                      "
                      maxlength="200"
                    /><small
                      class="bqatlas-field-error"
                      [id]="task.id + '-' + line.id + '-description-error'"
                      >{{ lineError(i, "description") }}</small
                    >
                  </td>
                  <td>
                    <bqatlas-decimal-input
                      [sheetEditor]="true"
                      [controlId]="task.id + '-' + line.id + '-quantity'"
                      [name]="'quantity-' + line.id"
                      [ariaLabel]="'Line ' + (i + 1) + ' quantity'"
                      [ngModel]="line.quantity"
                      (ngModelChange)="lineChange(line.id, 'quantity', $event)"
                      [scale]="3"
                      maximum="1000000"
                      [readonly]="!canEdit()"
                      [invalid]="!!lineError(i, 'quantity')"
                      [describedBy]="
                        task.id + '-' + line.id + '-quantity-error'
                      "
                    /><small
                      class="bqatlas-field-error"
                      [id]="task.id + '-' + line.id + '-quantity-error'"
                      >{{ lineError(i, "quantity") }}</small
                    >
                  </td>
                  <td>
                    <bqatlas-decimal-input
                      [sheetEditor]="true"
                      [controlId]="task.id + '-' + line.id + '-unitPrice'"
                      [name]="'price-' + line.id"
                      [ariaLabel]="'Line ' + (i + 1) + ' unit price'"
                      [ngModel]="line.unitPrice"
                      (ngModelChange)="lineChange(line.id, 'unitPrice', $event)"
                      [scale]="4"
                      maximum="1000000000"
                      [readonly]="!canEdit()"
                      [invalid]="!!lineError(i, 'unitPrice')"
                      [describedBy]="
                        task.id + '-' + line.id + '-unitPrice-error'
                      "
                    /><small
                      class="bqatlas-field-error"
                      [id]="task.id + '-' + line.id + '-unitPrice-error'"
                      >{{ lineError(i, "unitPrice") }}</small
                    >
                  </td>
                  <td class="quote-amount">{{ totals().lines[i] ?? "—" }}</td>
                  <td>
                    @if (canEdit()) {
                      <button
                        type="button"
                        class="quote-remove"
                        [attr.aria-label]="'Remove line ' + (i + 1)"
                        (click)="removeLine(line.id)"
                      >
                        ×
                      </button>
                    }
                  </td>
                </tr>
              }
            </tbody>
          </table>
        </div>
        <small class="bqatlas-field-error">{{
          errors()["lines"]?.join(" ")
        }}</small>
      </fieldset>
      <button type="submit" hidden>Save quote</button>
    </form>
    <footer class="quote-footer">
      <div>
        <strong>{{
          draft.dirty()
            ? "Unsaved changes"
            : draft.value().status === "submitted"
              ? "Submitted · Read only"
              : "Saved draft"
        }}</strong
        ><small
          >Each line rounds to 2 decimal places before the total is
          calculated.</small
        >
      </div>
      <div class="quote-total">
        <span
          >{{ draft.dirty() ? "Draft total" : "Total" }} ·
          {{ draft.value().currency }}</span
        ><strong>{{ totals().total ?? "Check amounts" }}</strong>
      </div>
    </footer>
  </section>`,
})
export class QuoteEditor {
  private readonly element = inject(ElementRef<HTMLElement>);
  private readonly injector = inject(Injector);
  private focusError() {
    afterNextRender(
      () => {
        if (
          this.controller.signal.aborted ||
          this.workspace.activeId() !== this.task.id
        )
          return;
        (this.element.nativeElement as HTMLElement)
          .querySelector<HTMLElement>('[aria-invalid="true"]')
          ?.focus();
      },
      { injector: this.injector },
    );
  }
  readonly task = inject(ATLAS_TASK) as WorkspaceTask<EditorTask>;
  readonly session = inject(AtlasSession);
  readonly workspace = inject(WorkspaceService);
  readonly crud = inject(CrudWorkspace);
  private readonly api = inject(AtlasApi);
  readonly draft = new RecordDraft<QuoteRecord>();
  readonly loading = signal(false);
  readonly errors = signal<Record<string, string[]>>({});
  readonly pendingSubmission = signal<{ key: string; version: string } | null>(
    null,
  );
  readonly canEdit = computed(
    () =>
      !this.pendingSubmission() &&
      this.session.has("sales.quotes.write") &&
      this.session.has("crm.customers.lookup") &&
      this.draft.value().status === "draft" &&
      (this.draft.capabilities()?.edit ?? true),
  );
  readonly canSubmit = computed(
    () =>
      this.session.has("sales.quotes.submit") &&
      this.draft.value().status === "draft" &&
      (this.draft.capabilities()?.commands.includes("submit") ?? true),
  );
  readonly totals = computed(() => quoteTotals(this.draft.value().lines));
  readonly selectedCustomer = computed(() =>
    [this.draft.value().customerCode, this.draft.value().customerName]
      .filter(Boolean)
      .join(" · "),
  );
  readonly products = new RestLookupProvider<ProductOption>(
    this.api,
    "/api/v1/engagement/products/lookup",
  );
  readonly productColumns = [
    { key: "code" as const, label: "SKU", width: 100 },
    { key: "name" as const, label: "Product" },
    { key: "unitPrice" as const, label: "Price", width: 100 },
    { key: "currency" as const, label: "Currency", width: 75 },
  ];
  readonly productKey = (row: ProductOption) => row.id;
  readonly productLabel = (row: ProductOption) => row.code + " · " + row.name;
  addProduct(row: ProductOption | null) {
    if (!row || !this.canEdit() || this.loading() || this.task.saving()) return;
    try {
      this.change("lines", appendCatalogProduct(this.draft.value(), row));
      this.task.error.set("");
    } catch (error) {
      this.task.error.set(
        error instanceof Error ? error.message : "Unable to add product.",
      );
    }
  }

  readonly customers = new RestLookupProvider<CustomerOption>(
    this.api,
    "/api/v1/crm/customers/lookup",
  );
  readonly customerColumns = [
    { key: "code" as const, label: "Code", width: 130 },
    { key: "name" as const, label: "Customer" },
  ];
  readonly customerKey = (row: CustomerOption) => row.id;
  readonly customerLabel = (row: CustomerOption) => `${row.code} · ${row.name}`;
  private readonly provider = new RestResourceProvider<QuoteRecord, QuoteInput>(
    this.api,
    endpoint,
  );
  private readonly controller = new AbortController();
  id = this.task.data().id;
  constructor() {
    this.draft.initialize({
      ...newQuote(),
      ...this.task.data().defaults,
    } as QuoteRecord);
    if (this.id) this.draft.dirty.set(false);
    this.task.lifecycle.save = () => this.persist();
    this.task.lifecycle.dispose = () => this.controller.abort();
    inject(DestroyRef).onDestroy(() => this.controller.abort());
    effect(() => this.task.dirty.set(this.draft.dirty()));
    if (this.id) void this.load();
  }
  change<K extends keyof QuoteRecord>(key: K, value: QuoteRecord[K]) {
    if (!this.canEdit() || this.loading() || this.task.saving()) return;
    this.draft.change(key, value);
    this.task.dirty.set(true);
  }
  chooseCustomer(row: CustomerOption | null) {
    this.change("customerId", row?.id ?? "");
    this.change("customerCode", row?.code ?? "");
    this.change("customerName", row?.name ?? "");
  }
  lineChange(
    id: string,
    key: "description" | "quantity" | "unitPrice",
    value: string,
  ) {
    this.change(
      "lines",
      this.draft
        .value()
        .lines.map((line) =>
          line.id === id ? { ...line, [key]: value } : line,
        ),
    );
  }
  addLine() {
    this.change("lines", [
      ...this.draft.value().lines,
      {
        id: crypto.randomUUID(),
        description: "",
        quantity: "1",
        unitPrice: "0",
      },
    ]);
  }
  removeLine(id: string) {
    this.change(
      "lines",
      this.draft.value().lines.filter((line) => line.id !== id),
    );
  }
  lineError(index: number, field: string) {
    return this.errors()[`lines[${index}].${field}`]?.join(" ") ?? "";
  }
  save() {
    return this.workspace.save(this.task.id);
  }
  saveShortcut(event: Event) {
    if (this.workspace.activeId() !== this.task.id) return;
    event.preventDefault();
    if (this.canEdit() && !this.loading()) void this.save();
  }
  private accept(record: RecordResult<QuoteRecord>) {
    this.id = record.data.id;
    this.draft.accept(record);
    this.task.dirty.set(false);
    this.task.data.set({ resource: "sales.quotes", id: this.id });
    this.workspace.rekey(this.task.id, `sales.quotes:${this.id}`);
    this.pendingSubmission.set(null);
    this.errors.set({});
  }
  async reload() {
    if (
      this.draft.dirty() &&
      !confirm("Discard your unsaved changes and reload this quote?")
    )
      return;
    if (this.id) await this.load();
    else {
      this.draft.initialize({
        ...newQuote(),
        ...this.task.data().defaults,
      } as QuoteRecord);
      this.errors.set({});
      this.task.error.set("");
    }
  }
  private async load() {
    this.loading.set(true);
    this.task.error.set("");
    try {
      const record = await this.provider.get(this.id!, this.controller.signal);
      if (!this.controller.signal.aborted) this.accept(record);
    } catch (error) {
      if (!this.controller.signal.aborted) this.fail(error);
    } finally {
      if (!this.controller.signal.aborted) this.loading.set(false);
    }
  }
  private async persist() {
    if (!this.canEdit() || this.loading())
      throw new Error("This quote cannot be edited.");
    this.errors.set({});
    const input = quoteInput(this.draft.value());
    const errors = validateQuote(input);
    if (Object.keys(errors).length) {
      this.errors.set(errors);
      this.focusError();
      throw new Error("Correct the highlighted quote fields.");
    }
    try {
      const record = this.id
        ? await this.provider.update(
            this.id,
            input,
            this.draft.version()!,
            this.controller.signal,
          )
        : await this.provider.create(input, this.controller.signal);
      if (!this.controller.signal.aborted) {
        this.accept(record);
        this.crud.changed();
      }
    } catch (error) {
      if (error instanceof ApiError) {
        this.errors.set(error.problem.errors ?? {});
        this.focusError();
      }
      throw error;
    }
  }
  async submit() {
    if (!this.canSubmit() || this.loading() || this.task.saving()) return;
    if ((this.draft.dirty() || !this.id) && !(await this.save())) return;
    const pending = this.pendingSubmission() ?? {
      key: crypto.randomUUID(),
      version: this.draft.version()!,
    };
    this.pendingSubmission.set(pending);
    this.task.saving.set(true);
    this.task.error.set("");
    try {
      const record = await this.api.request<RecordResult<QuoteRecord>>(
        `${endpoint}/${this.id}/submit`,
        "POST",
        undefined,
        this.controller.signal,
        pending.version,
        pending.key,
      );
      if (!this.controller.signal.aborted) {
        this.accept(record);
        this.crud.changed();
      }
    } catch (error) {
      if (!this.controller.signal.aborted) {
        if (error instanceof ApiError) {
          this.pendingSubmission.set(null);
          this.fail(error);
        } else
          this.task.error.set(
            "Could not confirm submission. Retry or reload the quote.",
          );
      }
    } finally {
      this.task.saving.set(false);
    }
  }
  async remove() {
    if (
      !this.id ||
      !this.draft.capabilities()?.delete ||
      !confirm("Delete this draft quote?")
    )
      return;
    this.task.saving.set(true);
    this.task.error.set("");
    try {
      await this.provider.delete(
        this.id,
        this.draft.version()!,
        this.controller.signal,
      );
      if (!this.controller.signal.aborted) {
        this.draft.dirty.set(false);
        this.task.dirty.set(false);
        this.task.saving.set(false);
        this.crud.changed();
        await this.workspace.requestClose(this.task.id);
      }
    } catch (error) {
      if (!this.controller.signal.aborted) this.fail(error);
    } finally {
      this.task.saving.set(false);
    }
  }
  private fail(error: unknown) {
    if (error instanceof ApiError) this.errors.set(error.problem.errors ?? {});
    this.task.error.set(
      error instanceof Error
        ? error.message
        : "Unable to complete the operation.",
    );
  }
}
export const salesFeature: CrudFeature = {
  resource: "sales.quotes",
  title: "Sales quotes",
  icon: "orders",
  writePermission: "sales.quotes.write",
  deletePermission: "sales.quotes.delete",
  defaults: {},
  listComponent: QuoteList,
  editorComponent: QuoteEditor,
};

interface ProductOption {
  id: string;
  code: string;
  name: string;
  unitPrice: string;
  currency: string;
  active: boolean;
}
