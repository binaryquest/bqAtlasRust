import { computed, signal } from "@angular/core";
export interface DemoCustomer {
  id: string;
  code: string;
  name: string;
  city: string;
  email: string;
  phone: string;
  terms: string;
  notes: string;
  active: boolean;
}
export interface CustomerIssue {
  id: string;
  field: "name" | "email";
  tab: "details" | "contacts";
  message: string;
}
const seeds: DemoCustomer[] = [
  {
    id: "C-101",
    code: "NORTH",
    name: "Northstar Supply",
    city: "Dhaka",
    email: "accounts@northstar.example",
    phone: "+880 1700 000101",
    terms: "Net 30",
    notes: "Deliveries accepted Monday–Thursday.",
    active: true,
  },
  {
    id: "C-102",
    code: "MERID",
    name: "Meridian Studio",
    city: "Chattogram",
    email: "hello@meridian.example",
    phone: "+880 1700 000102",
    terms: "Net 60",
    notes: "Contact before dispatch.",
    active: true,
  },
  {
    id: "C-103",
    code: "ACME",
    name: "Acme Components",
    city: "Sylhet",
    email: "office@acme.example",
    phone: "",
    terms: "Due on receipt",
    notes: "Account under review.",
    active: false,
  },
];
export function validateDemoCustomer(row: DemoCustomer): CustomerIssue[] {
  const issues: CustomerIssue[] = [];
  if (!row.name.trim())
    issues.push({
      id: row.id,
      field: "name",
      tab: "details",
      message: "Customer name is required.",
    });
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(row.email))
    issues.push({
      id: row.id,
      field: "email",
      tab: "contacts",
      message: "Enter a valid contact email.",
    });
  return issues;
}
/** Instance-owned local demonstration state; never calls the application's APIs. */
export class CustomerDemoStore {
  readonly saved = signal<DemoCustomer[]>(structuredClone(seeds));
  readonly drafts = signal<Record<string, DemoCustomer>>({});
  readonly selected = signal("C-101");
  readonly current = computed(
    () =>
      this.drafts()[this.selected()] ??
      this.saved().find((row) => row.id === this.selected())!,
  );
  readonly dirty = computed(() => Object.keys(this.drafts()).length > 0);
  readonly saving = signal(false);
  readonly readonly = signal(false);
  readonly failNext = signal(false);
  readonly issues = signal<CustomerIssue[]>([]);
  readonly error = signal("");
  readonly message = signal("");
  readonly focusRequest = signal(0);
  readonly history = signal<Record<string, string[]>>({});
  change<K extends keyof DemoCustomer>(field: K, value: DemoCustomer[K]) {
    if (this.saving() || this.readonly() || field === "id" || field === "code")
      return;
    const row = { ...this.current(), [field]: value };
    this.drafts.update((all) => {
      const next = { ...all };
      if (
        JSON.stringify(row) ===
        JSON.stringify(this.saved().find((x) => x.id === row.id))
      )
        delete next[row.id];
      else next[row.id] = row;
      return next;
    });
    this.issues.update((items) =>
      items.filter((issue) => !(issue.id === row.id && issue.field === field)),
    );
    this.error.set("");
    this.message.set("");
  }
  select(id: string) {
    if (!this.saving() && this.saved().some((row) => row.id === id))
      this.selected.set(id);
  }
  reset() {
    if (this.saving()) return;
    this.saved.set(structuredClone(seeds));
    this.drafts.set({});
    this.selected.set("C-101");
    this.issues.set([]);
    this.error.set("");
    this.message.set("Customer demo reset.");
    this.history.set({});
    this.failNext.set(false);
  }
  async save() {
    if (this.saving())
      throw new Error("A customer save is already in progress.");
    if (!this.dirty()) return;
    if (this.readonly())
      throw new Error("Turn off read-only mode before saving customer drafts.");
    const snapshot = structuredClone(this.drafts());
    const issues = Object.values(snapshot).flatMap(validateDemoCustomer);
    this.issues.set(issues);
    this.error.set("");
    this.message.set("");
    if (issues.length) {
      this.selected.set(issues[0].id);
      this.focusRequest.update((x) => x + 1);
      throw new Error(issues[0].message);
    }
    this.saving.set(true);
    try {
      await new Promise((resolve) => setTimeout(resolve, 400));
      if (this.failNext()) {
        this.failNext.set(false);
        throw new Error(
          "Simulated save failure. All customer drafts are retained.",
        );
      }
      this.saved.update((rows) => rows.map((row) => snapshot[row.id] ?? row));
      this.drafts.set({});
      this.history.update((all) => {
        const next = { ...all };
        for (const id of Object.keys(snapshot))
          next[id] = ["Saved locally in this demo", ...(next[id] ?? [])];
        return next;
      });
      this.message.set(
        `${Object.keys(snapshot).length} customer draft(s) saved locally.`,
      );
    } catch (error) {
      this.error.set(error instanceof Error ? error.message : "Save failed.");
      throw error;
    } finally {
      this.saving.set(false);
    }
  }
}
