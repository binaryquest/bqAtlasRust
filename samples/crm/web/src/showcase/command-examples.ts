import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  effect,
  inject,
  viewChild,
} from "@angular/core";
import { FormField, form, disabled, readonly } from "@angular/forms/signals";
import {
  ATLAS_TASK,
  WorkspaceService,
  AtlasButton,
  AtlasInput,
  AtlasPanel,
  AtlasCommandToolbar,
  AtlasPopupMenu,
  AtlasMenuButton,
  AtlasContextMenu,
  AtlasSplitButton,
  AtlasItemSelector,
  AtlasOption,
} from "@bqatlas/ui";
import { CommandDemoStore } from "./command-demo-store";
let instance = 0;
@Component({
  selector: "demo-commands",
  styleUrl: "./command-examples.css",
  imports: [
    FormField,
    AtlasButton,
    AtlasInput,
    AtlasPanel,
    AtlasCommandToolbar,
    AtlasPopupMenu,
    AtlasMenuButton,
    AtlasContextMenu,
    AtlasSplitButton,
    AtlasItemSelector,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="lab-section-intro">
      <div>
        <h3>Price lists & warehouse assignment</h3>
        <p>
          One command definition, four placements: toolbar, popup, context menu
          and split button.
        </p>
      </div>
      <button atlasButton (click)="openIndependent()">
        Open independent task
      </button>
    </div>
    <div class="lab-form-actions">
      <label
        ><input
          type="checkbox"
          [checked]="store.manage()"
          (change)="store.manage.set($any($event.target).checked)"
        />
        Simulate editing permission</label
      >
      <label
        ><input
          type="checkbox"
          [checked]="store.readonly()"
          (change)="store.readonly.set($any($event.target).checked)"
        />
        Read-only</label
      >
      <label
        ><input
          type="checkbox"
          [checked]="store.failNext()"
          (change)="store.failNext.set($any($event.target).checked)"
        />
        Fail next save</label
      >
      <button atlasButton [disabled]="store.saving()" (click)="store.reset()">
        Reset this demo
      </button>
    </div>
    <p class="atlas-muted">
      Local sample only. Editing permission is simulated; application
      permissions are unchanged. Narrow the window to move commands into More.
      Independent tasks keep separate drafts.
    </p>
    <atlas-command-toolbar
      [commands]="store.commands()"
      [permissions]="store.permissions()"
      label="Price-list actions"
      (command)="run($event)"
      >{{
        store.saving()
          ? "Saving…"
          : store.dirty()
            ? "Unsaved changes"
            : "Saved locally"
      }}</atlas-command-toolbar
    >
    <atlas-panel title="Price-list maintenance">
      <div panelToolbar class="lab-form-actions">
        <atlas-split-button
          [commands]="saveCommands()"
          primaryId="save"
          label="Save price list"
          [permissions]="store.permissions()"
          (command)="run($event)"
        />
        <atlas-menu-button
          label="Record actions"
          [commands]="store.commands()"
          [permissions]="store.permissions()"
          (command)="run($event)"
        />
      </div>
      <section
        class="command-context-card"
        tabindex="0"
        aria-label="Price-list context actions"
        [atlasContextMenu]="context"
      >
        <strong>{{ store.draft().title }}</strong>
        <p>
          Right-click here, or focus this card and press Shift+F10, for the same
          record actions. Escape returns focus.
        </p>
      </section>
      <atlas-popup-menu
        #context
        label="Price-list context actions"
        [commands]="store.commands()"
        [permissions]="store.permissions()"
        (command)="run($event)"
      />
      <div class="lab-form-grid">
        <label
          >Price-list name<input atlasInput [formField]="fields.title"
        /></label>
        <label
          >Standing desk · USD<input
            atlasInput
            inputmode="decimal"
            [formField]="fields.desk"
        /></label>
        <label
          >Task chair · USD<input
            atlasInput
            inputmode="decimal"
            [formField]="fields.chair"
        /></label>
      </div>
      <atlas-item-selector
        [controlId]="id"
        label="Warehouse assignment"
        availableLabel="Available warehouses"
        assignedLabel="Assigned warehouses"
        [options]="warehouses"
        [limit]="4"
        [formField]="fields.warehouses"
      />
      @if (store.error()) {
        <p class="atlas-edit-alert" role="alert">{{ store.error() }}</p>
      }
      <p role="status">{{ store.message() }}</p>
    </atlas-panel>
  `,
})
export class CommandExamples {
  readonly store = new CommandDemoStore();
  readonly id = `warehouse-assignment-${++instance}`;
  readonly workspace = inject(WorkspaceService);
  readonly fields = form(this.store.draft, (schema) => {
    disabled(schema, { when: () => this.store.saving() });
    readonly(schema, {
      when: () => this.store.readonly() || !this.store.manage(),
    });
  });
  readonly warehouses: AtlasOption[] = [
    {
      value: "dhaka",
      label: "DHK · Dhaka",
      description: "Central distribution",
    },
    {
      value: "chattogram",
      label: "CTG · Chattogram",
      description: "Port warehouse",
    },
    { value: "sylhet", label: "SYL · Sylhet", description: "Regional hub" },
    { value: "khulna", label: "KHL · Khulna", description: "Regional hub" },
    { value: "rajshahi", label: "RAJ · Rajshahi", description: "Regional hub" },
    {
      value: "bonded",
      label: "BND · Bonded storage",
      description: "Restricted",
      disabled: true,
    },
  ];
  constructor() {
    inject(DestroyRef).onDestroy(() => this.store.dispose());
  }
  saveCommands() {
    return this.store
      .commands()
      .filter((command) => ["save", "save-copy"].includes(command.id));
  }
  async run(id: string) {
    try {
      await this.store.execute(id);
    } catch (error) {
      this.store.error.set(
        error instanceof Error ? error.message : "Command failed.",
      );
    }
  }
  openIndependent() {
    this.workspace.open({ screen: "bqatlas.commands", data: {} });
  }
}
@Component({
  selector: "demo-command-task",
  imports: [CommandExamples],
  template: `<div class="command-task"><demo-commands /></div>`,
  styles: [".command-task {padding:18px; min-width:0;}"],
})
export class CommandDemoTask {
  readonly demo = viewChild(CommandExamples);
  readonly task = inject(ATLAS_TASK);
  constructor() {
    this.task.lifecycle.save = async () => {
      await this.demo()?.store.execute("save");
    };
    effect(() => this.task.dirty.set(this.demo()?.store.dirty() ?? false));
  }
}
