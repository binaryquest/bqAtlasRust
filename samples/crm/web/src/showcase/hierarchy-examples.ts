import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  inject,
  signal,
} from "@angular/core";
import {
  AtlasButton,
  AtlasCheckTree,
  AtlasTreeGrid,
  AtlasTreeNode,
  AtlasTreeGridColumn,
  AtlasPanel,
  AtlasLatestRequest,
} from "@bqatlas/ui";
const accounts: AtlasTreeNode[] = [
  {
    id: "assets",
    label: "1000 · Assets",
    values: { type: "Group", balance: "125400.30" },
    children: [
      {
        id: "cash",
        label: "1100 · Cash & bank",
        values: { type: "Posting", balance: "25400.10" },
      },
      {
        id: "receivables",
        label: "1200 · Receivables",
        values: { type: "Posting", balance: "100000.20" },
      },
      {
        id: "archived",
        label: "1300 · Legacy clearing",
        disabled: true,
        description: "Locked",
        values: { type: "Locked", balance: "0.00" },
      },
    ],
  },
  {
    id: "liabilities",
    label: "2000 · Liabilities",
    values: { type: "Group", balance: "40000.10" },
    children: [
      {
        id: "payables",
        label: "2100 · Trade payables",
        values: { type: "Posting", balance: "35000.00" },
      },
      {
        id: "accruals",
        label: "2200 · Accruals",
        values: { type: "Posting", balance: "5000.10" },
      },
    ],
  },
];
const bom = (): AtlasTreeNode[] => [
  {
    id: "desk",
    label: "DESK-100 · Standing desk",
    values: { qty: 1, unit: "each", cost: "420.00" },
    children: [
      {
        id: "top",
        label: "TOP-120 · Oak desktop",
        values: { qty: 1, unit: "each", cost: "180.00" },
      },
      {
        id: "frame",
        label: "FRAME-10 · Adjustable frame",
        hasChildren: true,
        description: "Expand to load components",
        values: { qty: 1, unit: "assembly", cost: "240.00" },
      },
    ],
  },
];
function updateNode(
  nodes: readonly AtlasTreeNode[],
  id: string,
  patch: Partial<AtlasTreeNode>,
): AtlasTreeNode[] {
  return nodes.map((node) =>
    node.id === id
      ? { ...node, ...patch }
      : node.children
        ? { ...node, children: updateNode(node.children, id, patch) }
        : node,
  );
}
@Component({
  selector: "demo-hierarchies",
  imports: [AtlasButton, AtlasCheckTree, AtlasTreeGrid, AtlasPanel],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="lab-section-intro">
      <div>
        <h3>Accounts & assemblies</h3>
        <p>
          Tri-state selection and multi-column hierarchies for ERP workflows.
        </p>
      </div>
    </div>
    <div class="lab-form-actions">
      <label
        ><input
          type="checkbox"
          [checked]="readonly()"
          (change)="readonly.set($any($event.target).checked)"
        />
        Read-only selection</label
      ><button atlasButton (click)="reset()">Reset hierarchy demo</button>
    </div>
    <atlas-panel title="Chart of accounts · report scope">
      <p class="atlas-muted">
        Space checks a posting account or its whole branch. Locked accounts are
        excluded. Arrows expand, collapse and move; Home/End move to the
        first/last visible row.
      </p>
      <atlas-check-tree
        [nodes]="accounts"
        label="Accounts included in report"
        [(checkedIds)]="checked"
        [(expandedIds)]="accountExpanded"
        [readonly]="readonly()"
      />
      <p role="status">
        {{ checked().length }} posting accounts included:
        {{ checked().join(", ") || "None" }}.
      </p>
      <atlas-tree-grid
        [nodes]="accounts"
        [columns]="accountColumns"
        label="Account balances"
        hierarchyLabel="Account"
        [(expandedIds)]="balanceExpanded"
        [(selectedId)]="selectedAccount"
        [readonly]="readonly()"
      />
      <p class="atlas-muted">
        Balances are sample values in USD. Parent balances are supplied values;
        the grid does not add parent and child balances together. Selected:
        {{ selectedAccount() || "None" }}.
      </p>
    </atlas-panel>
    <atlas-panel title="Bill of materials · DESK-100">
      <div panelToolbar class="lab-form-actions">
        <label
          ><input
            type="checkbox"
            [checked]="failNext()"
            (change)="failNext.set($any($event.target).checked)"
          />
          Fail next component load</label
        ><button atlasButton (click)="resetBom()">Reload assembly</button>
      </div>
      <p class="atlas-muted">
        Expand the adjustable frame to fetch its children. This is a delayed
        local simulation. After a failure, use Retry or Enter on the frame row.
        Quantities and costs are per parent assembly.
      </p>
      <atlas-tree-grid
        [nodes]="bom()"
        [columns]="bomColumns"
        label="Bill of materials"
        hierarchyLabel="Component"
        [(expandedIds)]="bomExpanded"
        [(selectedId)]="selectedPart"
        [readonly]="readonly()"
        (loadChildren)="load($event)"
      />
      <p role="status">Selected component: {{ selectedPart() || "None" }}.</p>
    </atlas-panel>
  `,
})
export class HierarchyExamples {
  readonly accounts = accounts;
  readonly readonly = signal(false);
  readonly checked = signal<string[]>(["cash"]);
  readonly accountExpanded = signal(["assets", "liabilities"]);
  readonly balanceExpanded = signal(["assets"]);
  readonly selectedAccount = signal<string | null>(null);
  readonly selectedPart = signal<string | null>(null);
  readonly bom = signal(bom());
  readonly bomExpanded = signal(["desk"]);
  readonly failNext = signal(false);
  private readonly request = new AtlasLatestRequest();
  readonly accountColumns: AtlasTreeGridColumn[] = [
    { key: "type", label: "Type" },
    { key: "balance", label: "Balance · USD", align: "right" },
  ];
  readonly bomColumns: AtlasTreeGridColumn[] = [
    { key: "qty", label: "Quantity", align: "right" },
    { key: "unit", label: "Unit" },
    { key: "cost", label: "Unit cost · USD", align: "right" },
  ];
  constructor() {
    inject(DestroyRef).onDestroy(() => this.request.cancel());
  }
  resetBom() {
    this.request.cancel();
    this.bom.set(bom());
    this.bomExpanded.set(["desk"]);
    this.selectedPart.set(null);
  }
  reset() {
    this.resetBom();
    this.checked.set(["cash"]);
    this.accountExpanded.set(["assets", "liabilities"]);
    this.balanceExpanded.set(["assets"]);
    this.selectedAccount.set(null);
    this.failNext.set(false);
    this.readonly.set(false);
  }
  load(node: AtlasTreeNode) {
    const fail = this.failNext();
    this.failNext.set(false);
    this.bom.update((nodes) =>
      updateNode(nodes, node.id, { loading: true, error: "" }),
    );
    void this.request.run(
      async () => {
        await new Promise((resolve) => setTimeout(resolve, 650));
        if (fail) throw new Error("Components unavailable.");
        return [
          {
            id: "legs",
            label: "LEG-20 · Telescopic leg",
            values: { qty: 2, unit: "each", cost: "80.00" },
          },
          {
            id: "motor",
            label: "MOTOR-24 · Lift motor",
            values: { qty: 1, unit: "each", cost: "80.00" },
          },
        ];
      },
      (children) =>
        this.bom.update((nodes) =>
          updateNode(nodes, node.id, {
            children,
            loading: false,
            error: "",
            description: "",
          }),
        ),
      () =>
        this.bom.update((nodes) =>
          updateNode(nodes, node.id, {
            loading: false,
            error: "Components unavailable.",
          }),
        ),
    );
  }
}
