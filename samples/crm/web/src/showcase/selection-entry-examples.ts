import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  computed,
  inject,
  signal,
} from "@angular/core";
import { JsonPipe } from "@angular/common";
import {
  FormField,
  form,
  required,
  min,
  validate,
  disabled,
  readonly,
} from "@angular/forms/signals";
import {
  AtlasMultiSelect,
  AtlasAutocomplete,
  AtlasCheckboxGroup,
  AtlasRadioGroup,
  AtlasDateInput,
  AtlasDateRangeInput,
  AtlasDecimalInput,
  AtlasValidationSummary,
  AtlasPanel,
  AtlasField,
  AtlasButton,
  AtlasOption,
  dateRangeError,
  validCalendarDate,
  AtlasValidationIssue,
} from "@bqatlas/ui";
@Component({
  selector: "demo-selection-entry",
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    JsonPipe,
    FormField,
    AtlasMultiSelect,
    AtlasAutocomplete,
    AtlasCheckboxGroup,
    AtlasRadioGroup,
    AtlasDateInput,
    AtlasDateRangeInput,
    AtlasDecimalInput,
    AtlasValidationSummary,
    AtlasPanel,
    AtlasField,
    AtlasButton,
  ],
  template: `<div class="lab-section-intro">
      <div>
        <h3>Selection & business entry</h3>
        <p>
          Build a purchase request using signal-based controls. Change values,
          test limits, and review validation together.
        </p>
      </div>
    </div>
    <atlas-validation-summary
      [issues]="issues()"
      (fieldRequested)="focusField($event)"
    />
    <atlas-panel title="People & preferences"
      ><div class="form-grid">
        <atlas-field label="Approvers (up to 2)" controlId="phase2-approvers"
          ><atlas-multi-select
            controlId="phase2-approvers"
            label="Approvers"
            [options]="people"
            [limit]="2"
            [formField]="fields.approvers"
        /></atlas-field>
        <atlas-field label="Requested by" controlId="phase2-requester"
          ><atlas-autocomplete
            controlId="phase2-requester"
            label="Requester"
            [options]="people"
            [formField]="fields.requester"
        /></atlas-field>
        <atlas-checkbox-group
          controlId="phase2-services"
          label="Additional services"
          [options]="services"
          [formField]="fields.services"
        />
        <atlas-radio-group
          controlId="phase2-priority"
          label="Priority"
          [options]="priorities"
          [formField]="fields.priority"
        /></div
    ></atlas-panel>
    <atlas-panel title="Dates & amounts"
      ><div class="form-grid">
        <atlas-field label="Required by" controlId="phase2-date"
          ><atlas-date-input
            controlId="phase2-date"
            label="Required by"
            [formField]="fields.requiredBy"
        /></atlas-field>
        <atlas-date-range
          controlId="phase2-period"
          label="Delivery window"
          [formField]="fields.period"
        />
        <atlas-field label="Budget" controlId="phase2-budget"
          ><atlas-decimal-input
            controlId="phase2-budget"
            label="Budget"
            [locale]="locale()"
            currency="EUR"
            [formField]="fields.budget"
            (parseErrorChange)="badBudget.set($event)"
        /></atlas-field>
        <atlas-field label="Weight (kg)" controlId="phase2-weight"
          ><atlas-decimal-input
            controlId="phase2-weight"
            label="Weight"
            [locale]="locale()"
            [fractionDigits]="3"
            [formField]="fields.weight"
            (parseErrorChange)="badWeight.set($event)"
        /></atlas-field>
      </div>
      <div class="lab-form-actions">
        <button
          atlasButton
          (click)="locale.set(locale() === 'en-US' ? 'de-DE' : 'en-US')"
        >
          Locale: {{ locale() }}
        </button>
      </div></atlas-panel
    >
    <div class="lab-form-actions">
      <button atlasButton variant="primary" (click)="validateForm()">
        Validate request</button
      ><button atlasButton (click)="reset()">Reset request</button
      ><button atlasButton (click)="locked.update(invert)">
        {{ locked() ? "Enable request" : "Disable request" }}</button
      ><button atlasButton (click)="viewOnly.update(invert)">
        {{ viewOnly() ? "Edit request" : "Read-only request" }}
      </button>
    </div>
    @if (message()) {
      <p role="status" class="lab-note">{{ message() }}</p>
    }
    <atlas-panel title="Signal model">
      <pre class="lab-value-preview">{{ data() | json }}</pre>
      <p class="lab-note">
        Dates are YYYY-MM-DD without timezone conversion. Amounts remain
        numbers; only their display uses a locale.
      </p></atlas-panel
    >`,
})
export class SelectionEntryExamples {
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  readonly initial = {
    approvers: [] as string[],
    requester: null as string | null,
    services: [] as string[],
    priority: "normal" as string | null,
    requiredBy: "",
    period: { start: "", end: "" },
    budget: null as number | null,
    weight: 1.25 as number | null,
  };
  readonly data = signal(structuredClone(this.initial));
  readonly locked = signal(false);
  readonly viewOnly = signal(false);
  readonly locale = signal("en-US");
  readonly badBudget = signal(false);
  readonly badWeight = signal(false);
  readonly submitted = signal(false);
  readonly message = signal("");
  readonly invert = (value: boolean) => !value;
  readonly fields = form(this.data, (s) => {
    validate(s.approvers, (ctx) =>
      ctx.value().length
        ? null
        : { kind: "required", message: "Choose at least one approver." },
    );
    required(s.requester);
    required(s.requiredBy);
    validate(s.requiredBy, (ctx) =>
      !ctx.value() || validCalendarDate(ctx.value())
        ? null
        : { kind: "date", message: "Enter a valid required-by date." },
    );
    required(s.budget);
    min(s.budget, 0);
    min(s.weight, 0);
    validate(s.period, (ctx) => {
      const message = dateRangeError(ctx.value());
      return message ? { kind: "dateRange", message } : null;
    });
    disabled(s, { when: () => this.locked() });
    readonly(s, { when: () => this.viewOnly() });
  });
  readonly people: AtlasOption[] = [
    { label: "Alex Morgan", value: "alex", description: "Operations lead" },
    { label: "Jamie Chen", value: "jamie", description: "Finance manager" },
    { label: "Sam Rivera", value: "sam", description: "Purchasing specialist" },
    {
      label: "Taylor Brooks",
      value: "taylor",
      description: "Archived account",
      disabled: true,
    },
  ];
  readonly services: AtlasOption[] = [
    { label: "Installation", value: "install" },
    { label: "Extended warranty", value: "warranty" },
    { label: "Express delivery", value: "express", disabled: true },
  ];
  readonly priorities: AtlasOption[] = [
    {
      label: "Normal",
      value: "normal",
      description: "Standard approval workflow",
    },
    { label: "Urgent", value: "urgent", description: "Expedited review" },
    { label: "Critical", value: "critical", disabled: true },
  ];
  readonly issues = computed(() => {
    if (!this.submitted() || this.locked() || this.viewOnly()) return [];
    const issues: AtlasValidationIssue[] = [];
    if (this.fields.approvers().invalid())
      issues.push({
        id: "phase2-approvers",
        message: "Choose at least one approver.",
      });
    if (this.fields.requester().invalid())
      issues.push({
        id: "phase2-requester",
        message: "Select a requester from the suggestions.",
      });
    if (this.fields.requiredBy().invalid())
      issues.push({
        id: "phase2-date",
        message: "Enter the required-by date.",
      });
    const period = dateRangeError(this.data().period);
    if (period) issues.push({ id: "phase2-period", message: period });
    if (this.fields.budget().invalid() || this.badBudget())
      issues.push({
        id: "phase2-budget",
        message: "Enter a valid non-negative budget.",
      });
    if (this.fields.weight().invalid() || this.badWeight())
      issues.push({
        id: "phase2-weight",
        message: "Enter a valid non-negative weight.",
      });
    return issues;
  });
  focusField(id: string) {
    this.host.nativeElement.querySelector<HTMLElement>(`[id="${id}"]`)?.focus();
  }
  validateForm() {
    this.submitted.set(true);
    this.fields().markAsTouched();
    this.message.set(
      this.locked()
        ? "Enable the request to validate."
        : this.viewOnly()
          ? "This request is read-only."
          : this.issues().length
            ? "Review the highlighted fields."
            : "Request is valid. No data was sent.",
    );
  }
  reset() {
    this.fields().reset(structuredClone(this.initial));
    this.badBudget.set(false);
    this.badWeight.set(false);
    this.submitted.set(false);
    this.message.set("");
  }
}
