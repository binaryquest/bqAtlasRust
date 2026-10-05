export const controlDocs = [
  {
    id: "button",
    title: "Button",
    group: "Foundations",
    imports: "AtlasButton",
    description:
      "Actions with consistent primary, secondary, ghost and danger treatments.",
    setup: "count = signal(0);\nincrement = (value: number) => value + 1;",
    template:
      '<button atlasButton variant="primary" type="button" (click)="count.update(increment)">Save</button>',
    api: "variant: primary | secondary | ghost | danger. Native disabled, type and click retain their normal behavior. Define increment = (n: number) => n + 1 in the component.",
    keyboard:
      'Tab focuses; Enter or Space activates. Use type="button" inside forms unless submitting.',
  },
  {
    id: "input",
    title: "Text input",
    group: "Inputs",
    imports: "AtlasInput",
    description:
      "Style a native input without replacing its browser semantics.",
    setup: "name = '';",
    template:
      '<label for="name">Customer name</label>\n<input atlasInput id="name" [(ngModel)]="name" required maxlength="200" />',
    api: "AtlasInput is a styling directive. Import FormsModule for ngModel; native required, maxlength, disabled and autocomplete are supported.",
    keyboard:
      "Standard native text-editing and Tab behavior. Always supply a visible label.",
  },
  {
    id: "textarea",
    title: "Textarea",
    group: "Inputs",
    imports: "AtlasTextarea",
    description: "Long notes with a character counter and a signal value.",
    setup: "notes = signal('');",
    template:
      '<atlas-textarea controlId="notes" ariaLabel="Notes" [(value)]="notes" [maxLength]="2000" />',
    api: "value / valueChange: string; rows: number; maxLength: number; showCount: boolean. Supports disabled, readonly, invalid, touched, describedBy and Signal Forms.",
    keyboard: "Native textarea keyboard behavior; touch emits on blur.",
  },
  {
    id: "select",
    title: "Select",
    group: "Selection",
    imports: "AtlasSelect",
    description: "Search a small option list and store its key.",
    setup:
      "currency = 'USD';\noptions = [{value:'USD', label:'US Dollar'}, {value:'EUR', label:'Euro'}];",
    template:
      '<atlas-select controlId="currency" ariaLabel="Currency" [options]="options" [(ngModel)]="currency" />',
    api: "CVA: use ngModel or reactive forms, with FormsModule or ReactiveFormsModule. options contain value, label and optional disabled/description. clearable and loading configure the picker.",
    keyboard:
      "Arrow keys navigate; Enter selects; Escape dismisses. Tab moves onward.",
  },
  {
    id: "lookup",
    title: "Multi-column lookup",
    group: "Selection",
    imports: "AtlasLookup",
    description:
      "Choose a record using a searchable grid with multiple columns.",
    setup:
      "selected = signal<string | null>(null);\nrows = [{id:'1', code:'CONSULT', name:'Consulting', price:'125.00'}];\ncolumns = [{key:'code' as const, label:'SKU'}, {key:'name' as const, label:'Product'}, {key:'price' as const, label:'Price'}];\nkey = (row: typeof this.rows[number]) => row.id;\ncaption = (row: typeof this.rows[number]) => row.name;",
    template:
      '<atlas-lookup controlId="product" ariaLabel="Product" [rows]="rows" [columns]="columns" [recordKey]="key" [displayWith]="caption" [(value)]="selected" />',
    api: "rows, columns, recordKey and displayWith are required. value is the selected key. recordSelected emits the row. remote, queryChange, page, total, loading and error support remote data.",
    keyboard:
      "Arrow keys navigate records; Enter selects; Escape dismisses. Clear and dropdown have separate focus targets.",
  },
  {
    id: "reference",
    title: "Related-record lookup",
    group: "Integration",
    imports: "ReferenceLookup, RestLookupProvider, AtlasApi",
    description:
      "Search, create and open records without abandoning the parent draft. Used by customers in quotes and CRM forms.",
    setup:
      "api = inject(AtlasApi);\nprovider = new RestLookupProvider<Customer>(this.api, '/api/v1/crm/customers/lookup');\nselected = signal<string | null>(null);\ncolumns = [{key:'code' as const,label:'Code'}, {key:'name' as const,label:'Customer'}];\nkey = (row: Customer) => row.id;\ncaption = (row: Customer) => row.code + ' \u00b7 ' + row.name;",
    template:
      '<bqatlas-reference-lookup controlId="customer" label="Customer" resource="crm.customers" [provider]="provider" [columns]="columns" [recordKey]="key" [displayWith]="caption" [value]="selected()" (recordSelected)="selected.set($event?.id ?? null)" />',
    api: "Import from @bqatlas/angular. Define Customer {id:string;code:string;name:string}. Register the resource with CrudWorkspace; session permissions control create/open actions. Provider.resolve refreshes saved captions; contextKey invalidates stale returns. Custom editors require their own related-record support.",
    keyboard:
      "Search more opens a paged dialog. Create opens a separate editor; Save & select returns to the field. Open is available for read-only parent records.",
  },
  {
    id: "multi",
    title: "Multi-select",
    group: "Selection",
    imports: "AtlasMultiSelect",
    description:
      "Select several tags with a searchable list and optional limit.",
    setup:
      "tags = signal<string[]>([]);\noptions = [{value:'partner',label:'Partner'}, {value:'priority',label:'Priority'}];",
    template:
      '<atlas-multi-select controlId="tags" label="Account tags" [options]="options" [(value)]="tags" [limit]="2" />',
    api: "value / valueChange: string[]; options: AtlasOption[]; limit: number (0 means unlimited); disabled, readonly and loading supported.",
    keyboard:
      "Tab reaches each checkbox and footer action; Space toggles. Selected chips have individually labelled remove buttons.",
  },
  {
    id: "radio",
    title: "Radio group",
    group: "Selection",
    imports: "AtlasRadioGroup",
    description:
      "A visible single choice for short lists such as activity type.",
    setup:
      "kind = signal<string | null>('Call');\noptions = [{value:'Call',label:'Call'}, {value:'Meeting',label:'Meeting'}];",
    template:
      '<atlas-radio-group controlId="kind" label="Activity type" [options]="options" [(value)]="kind" />',
    api: "value / valueChange: string | null; options, label and controlId; disabled and readonly; invalid is announced when touched.",
    keyboard:
      "Native radio keyboard behavior: Arrow keys choose within the group, Space selects.",
  },
  {
    id: "checkbox",
    title: "Checkbox group",
    group: "Selection",
    imports: "AtlasCheckboxGroup",
    description: "Independent choices with optional selection limits.",
    setup:
      "channels = signal<string[]>(['email']);\noptions = [{value:'email',label:'Email'}, {value:'phone',label:'Phone'}];",
    template:
      '<atlas-checkbox-group controlId="channels" label="Contact channels" [options]="options" [(value)]="channels" />',
    api: "value / valueChange: string[]; options: AtlasOption[]; limit: number; disabled, readonly, invalid and touched.",
    keyboard: "Tab focuses each checkbox; Space toggles its value.",
  },
  {
    id: "toggle",
    title: "Toggle",
    group: "Selection",
    imports: "AtlasToggle",
    description:
      "An explicit on/off setting such as an active catalog product.",
    setup: "active = signal(true);",
    template:
      '<atlas-toggle controlId="active" label="Available for new business" [(checked)]="active" />',
    api: "checked / checkedChange: boolean; label and controlId required; disabled, readonly and describedBy supported. Compatible with Signal Forms checkbox controls.",
    keyboard:
      "Tab focuses the switch; Enter or Space toggles. State is exposed with aria-checked.",
  },
  {
    id: "date",
    title: "Date input",
    group: "Inputs",
    imports: "AtlasDateInput",
    description: "A local calendar date with no implicit timezone conversion.",
    setup: "date = signal('2026-10-01');",
    template:
      '<atlas-date-input controlId="due" label="Due date" [(value)]="date" minDate="1900-01-01" />',
    api: "value / valueChange: YYYY-MM-DD string; minDate, maxDate; disabled, readonly, invalid and touched. Validate business date constraints on the server too.",
    keyboard:
      "Uses the browser date input and its native picker; keyboard behavior follows the platform.",
  },
  {
    id: "decimal",
    title: "Exact decimal",
    group: "Inputs",
    imports: "AtlasDecimalTextInput",
    description:
      "Keep financial amounts as decimal strings across the API boundary.",
    setup: "price = '125.0000';",
    template:
      '<bqatlas-decimal-input controlId="price" ariaLabel="Unit price" [(ngModel)]="price" [scale]="4" maximum="1000000000" />',
    api: "Import from @bqatlas/angular with FormsModule. ngModel is a decimal string, never a JavaScript number. scale and maximum constrain parsing; server validates independently.",
    keyboard:
      "Native text editing; inputmode supports decimal keyboards. The UI-only AtlasDecimalInput uses number values and is a different control.",
  },
  {
    id: "tabs",
    title: "Tabs",
    group: "Layout",
    imports: "AtlasTabs, AtlasTab",
    description:
      "Divide a complex form while retaining visited panels and their drafts.",
    setup: "section = signal('details');",
    template:
      '<atlas-tabs label="Customer sections" [(selected)]="section">\n  <ng-template atlasTab="details" label="Details">Customer details</ng-template>\n  <ng-template atlasTab="sales" label="Sales">Sales history</ng-template>\n</atlas-tabs>',
    api: "selected / selectedChange: tab ID. atlasTab directive supplies id, label and disabled. Panels instantiate on first visit and remain mounted.",
    keyboard:
      "Arrow Left/Right, Home and End move between tabs. Tab enters the active panel.",
  },
  {
    id: "accordion",
    title: "Accordion section",
    group: "Layout",
    imports: "AtlasAccordionSection",
    description: "Collapsible supporting details within a form.",
    setup: "expanded = signal(true);",
    template:
      '<atlas-accordion-section label="Contact preferences" [(expanded)]="expanded">\n  <p>Preferred contact: email</p>\n</atlas-accordion-section>',
    api: "label: required string; expanded / expandedChange: boolean. Project any content; collapsing preserves its state.",
    keyboard:
      "Header is a native button. Enter or Space toggles aria-expanded.",
  },
  {
    id: "split",
    title: "Split pane",
    group: "Layout",
    imports: "AtlasSplitPane",
    description:
      "Resizable master/detail layout that stacks in a narrow workspace.",
    setup: "ratio = signal(45);",
    template:
      '<atlas-split-pane primaryLabel="Customers" secondaryLabel="Overview" [(ratio)]="ratio">\n  <div atlasSplitPrimary>Customer directory</div>\n  <div atlasSplitSecondary>Selected customer</div>\n</atlas-split-pane>',
    api: "ratio: percentage; minimum, maximum, breakpoint; collapsed: boolean. Panels use atlasSplitPrimary and atlasSplitSecondary projection attributes.",
    keyboard:
      "Focus the separator: Arrow keys resize, Shift uses larger steps, Home/End go to the limits. Hide/Show toggles the primary pane.",
  },
  {
    id: "table",
    title: "Data table",
    group: "Data",
    imports: "AtlasTable",
    description:
      "Sortable, searchable records with pagination and column options.",
    setup:
      "rows = [{id:'1',name:'Northwind',status:'Active'}, {id:'2',name:'Alpine',status:'Prospect'}];\ncolumns = [{key:'name' as const,label:'Customer'}, {key:'status' as const,label:'Status'}];",
    template:
      '<atlas-table [rows]="rows" [columns]="columns" keyField="id" label="Customers" [filterable]="true" [columnManage]="true" [multiSort]="true" [selectable]="true" />',
    api: "Local mode manages sorting/filtering/paging. Server mode emits queryChange; provide total, loading and error. rowActivated opens a record. Decimal string columns need an explicit numeric comparator or server sorting.",
    keyboard:
      "Tab reaches search and toolbar. Rows support keyboard activation; column headers expose sorting state.",
  },
  {
    id: "dialog",
    title: "Dialog",
    group: "Feedback",
    imports: "AtlasDialog, AtlasButton",
    description: "A modal for short, focused interactions with managed focus.",
    setup: "open = signal(false);",
    template:
      '<button atlasButton type="button" (click)="open.set(true)">Open dialog</button>\n<atlas-dialog title="Review account" [(open)]="open">\n  <p>Check the customer details before continuing.</p>\n  <button atlasButton type="button" (click)="open.set(false)">Done</button>\n</atlas-dialog>',
    api: "title: required; open / openChange: boolean; busy prevents dismissal during work. Native dialog lives in the top layer.",
    keyboard:
      "Focus stays within the modal. Escape closes unless busy; the opener regains focus.",
  },
] as const;
