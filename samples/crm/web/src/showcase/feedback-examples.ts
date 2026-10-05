import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  inject,
  signal,
} from "@angular/core";
import {
  AtlasButton,
  AtlasNotice,
  AtlasProgress,
  AtlasStatusBar,
  AtlasLoadingRegion,
  AtlasMessageBox,
  AtlasPopover,
  AtlasTooltip,
  WorkspaceService,
} from "@bqatlas/ui";
import type { AtlasMessageResult } from "@bqatlas/ui";
import { FeedbackDemoStore } from "./feedback-store";
@Component({
  selector: "demo-feedback",
  imports: [
    AtlasButton,
    AtlasNotice,
    AtlasProgress,
    AtlasStatusBar,
    AtlasLoadingRegion,
    AtlasMessageBox,
    AtlasPopover,
    AtlasTooltip,
  ],
  styleUrl: "./feedback-examples.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: ` <section class="feedback-demo">
    <header>
      <div>
        <span class="feedback-eyebrow">FEEDBACK & GUIDANCE</span>
        <h3>Inventory import desk</h3>
        <p>
          Clear confirmations, contextual help and recoverable background work.
        </p>
      </div>
      <button
        atlasButton
        (click)="workspace.open({ screen: 'bqatlas.feedback', data: {} })"
      >
        Open independent import
      </button>
    </header>
    @if (showIntro()) {
      <atlas-notice
        title="A safe place to try the workflow"
        message="These sample rows stay in this task. No files are uploaded and no inventory records are changed."
        [dismissible]="true"
        (dismissed)="showIntro.set(false)"
      />
    }
    <section class="feedback-card">
      <header>
        <div>
          <h4>{{ store.name() }}</h4>
          <p>3 sample inventory rows · Local demonstration</p>
        </div>
        <div class="feedback-actions">
          <button
            atlasButton
            [disabled]="store.busy()"
            (click)="show('prompt')"
          >
            Rename batch</button
          ><atlas-tooltip
            label="About batch names"
            text="A batch name identifies this local preview. Use 1–80 characters. Renaming does not change inventory records."
          />
        </div>
      </header>
      <div class="feedback-toolbar">
        <div class="feedback-actions">
          <button
            atlasButton
            variant="primary"
            [disabled]="store.busy() || store.phase() === 'complete'"
            (click)="show('confirm')"
          >
            Validate & import</button
          ><button
            atlasButton
            [disabled]="!store.busy()"
            (click)="store.cancel()"
          >
            Cancel import</button
          ><button
            atlasButton
            [disabled]="store.busy()"
            (click)="show('reset')"
          >
            Reset preview
          </button>
        </div>
        <label
          ><input
            type="checkbox"
            [checked]="store.failNext()"
            [disabled]="store.busy()"
            (change)="store.failNext.set($any($event.target).checked)"
          />
          Fail next import</label
        >
      </div>
      <div class="feedback-help">
        <atlas-popover label="How imports work" title="Validate, then commit"
          ><p>
            Validation checks the entire sample before import begins. The result
            is committed only after every row succeeds.
          </p>
          <p>
            If work fails or is cancelled, the preview stays available for
            another attempt.
          </p>
          <button atlasButton (click)="detailsRead.set(true)">
            Mark guidance as read
          </button>
          @if (detailsRead()) {
            <p role="status">Guidance read for this task.</p>
          }</atlas-popover
        ><button atlasButton (click)="show('alert')">
          Show validation summary
        </button>
      </div>
      <atlas-loading-region
        [busy]="store.busy()"
        label="Import preview"
        [message]="
          store.phase() === 'validating'
            ? 'Checking codes and quantities…'
            : 'Import in progress…'
        "
        ><div class="atlas-table-scroll">
          <table class="atlas-table">
            <caption>
              Inventory preview
            </caption>
            <thead>
              <tr>
                <th scope="col">Code</th>
                <th scope="col">Product</th>
                <th scope="col">Quantity</th>
                <th scope="col">Details</th>
              </tr>
            </thead>
            <tbody>
              @for (row of store.rows(); track row.code) {
                <tr>
                  <td>{{ row.code }}</td>
                  <td>{{ row.name }}</td>
                  <td>{{ row.quantity }}</td>
                  <td>
                    <button atlasButton (click)="inspected.set(row.code)">
                      Inspect {{ row.code }}
                    </button>
                  </td>
                </tr>
              }
            </tbody>
          </table>
        </div></atlas-loading-region
      >
      @if (inspected()) {
        <p class="feedback-muted">Selected preview: {{ inspected() }}</p>
      }
      @if (store.busy()) {
        <atlas-progress
          [value]="store.phase() === 'validating' ? null : store.completed()"
          [max]="store.rows().length"
          [label]="
            store.phase() === 'validating'
              ? 'Validating sample'
              : 'Importing inventory'
          "
        />
      }
      @if (store.phase() === "error") {
        <atlas-notice
          tone="danger"
          title="Import did not complete"
          [message]="store.error()"
          [urgent]="true"
        /><button atlasButton variant="primary" (click)="store.start()">
          Retry import
        </button>
      }
      @if (store.phase() === "complete") {
        <atlas-notice
          tone="success"
          title="Sample import complete"
          [message]="
            store.imported().length +
            ' rows are in the local result. No backend records were created.'
          "
        />
      }
      @if (store.phase() === "cancelled") {
        <atlas-notice
          tone="warning"
          title="Import cancelled"
          message="Your preview is intact. You can validate and import again."
        />
      }
      <atlas-status-bar
        [message]="store.message()"
        [tone]="
          store.phase() === 'error'
            ? 'danger'
            : store.phase() === 'complete'
              ? 'success'
              : 'info'
        "
        >Preview rows: {{ store.rows().length }} · Local result:
        {{ store.imported().length }}</atlas-status-bar
      >
    </section>
    <p class="feedback-muted">
      Escape dismisses help and dialogs. Tab reaches every action. Cancel
      remains available while the preview is locked. Closing this task cancels
      its local work.
    </p>
    <atlas-message-box
      [(open)]="dialogOpen"
      [kind]="
        dialogKind() === 'prompt'
          ? 'prompt'
          : dialogKind() === 'alert'
            ? 'alert'
            : 'confirm'
      "
      [title]="
        dialogKind() === 'prompt'
          ? 'Rename import batch'
          : dialogKind() === 'alert'
            ? 'Sample validation rules'
            : dialogKind() === 'reset'
              ? 'Reset this preview?'
              : 'Import sample inventory?'
      "
      [message]="
        dialogKind() === 'prompt'
          ? 'Choose a descriptive name for this local batch.'
          : dialogKind() === 'alert'
            ? 'Codes must be unique and quantities must be non-negative whole numbers. This sample contains three valid rows.'
            : dialogKind() === 'reset'
              ? 'This clears the local result and restores the sample rows.'
              : 'The sample will be validated and imported locally. No backend records will change.'
      "
      promptLabel="Batch name"
      [maxLength]="80"
      [(value)]="prompt"
      [acceptLabel]="
        dialogKind() === 'prompt'
          ? 'Rename'
          : dialogKind() === 'alert'
            ? 'Got it'
            : dialogKind() === 'reset'
              ? 'Reset preview'
              : 'Start import'
      "
      [danger]="dialogKind() === 'reset'"
      (result)="answer($event)"
    />
  </section>`,
})
export class FeedbackExamples {
  readonly store = new FeedbackDemoStore();
  readonly workspace = inject(WorkspaceService);
  readonly showIntro = signal(true);
  readonly detailsRead = signal(false);
  readonly inspected = signal("");
  readonly dialogOpen = signal(false);
  readonly dialogKind = signal<"alert" | "confirm" | "prompt" | "reset">(
    "confirm",
  );
  readonly prompt = signal("");
  constructor() {
    inject(DestroyRef).onDestroy(() => this.store.dispose());
  }
  show(kind: "alert" | "confirm" | "prompt" | "reset") {
    if (this.store.busy() && kind !== "alert") return;
    this.dialogKind.set(kind);
    this.prompt.set(this.store.name());
    this.dialogOpen.set(true);
  }
  answer(result: AtlasMessageResult) {
    if (result.action !== "accept") return;
    switch (this.dialogKind()) {
      case "prompt":
        this.store.rename(result.value);
        break;
      case "confirm":
        void this.store.start();
        break;
      case "reset":
        this.store.reset();
        this.inspected.set("");
        break;
    }
  }
}
@Component({
  selector: "demo-feedback-task",
  imports: [FeedbackExamples],
  template: `<div style="padding:18px;min-width:0"><demo-feedback /></div>`,
})
export class FeedbackDemoTask {}
