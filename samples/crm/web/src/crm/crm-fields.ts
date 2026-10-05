import { Component, input, inject } from "@angular/core";
import {
  AtlasTextarea,
  AtlasRadioGroup,
  AtlasToggle,
  AtlasDateInput,
} from "@bqatlas/ui";
import {
  AtlasApi,
  FieldRendererContext,
  ReferenceLookup,
  RestLookupProvider,
} from "@bqatlas/angular";
import { CustomerOption } from "../sales/quote-model";
@Component({
  selector: "app-crm-customer-field",
  imports: [ReferenceLookup],
  template: `<bqatlas-reference-lookup
    [controlId]="context().controlId"
    label="Customer"
    resource="crm.customers"
    [provider]="provider"
    [columns]="columns"
    [recordKey]="key"
    [displayWith]="display"
    [value]="value()"
    [selectedText]="caption()"
    [disabled]="context().disabled"
    [invalid]="context().errors.length > 0"
    [describedBy]="context().errorId"
    (recordSelected)="context().change($event?.id || '')"
  />`,
})
export class CrmCustomerField {
  readonly context = input.required<FieldRendererContext>();
  readonly provider = new RestLookupProvider<CustomerOption>(
    inject(AtlasApi),
    "/api/v1/crm/customers/lookup",
  );
  readonly columns = [
    { key: "code" as const, label: "Code", width: 110 },
    { key: "name" as const, label: "Customer" },
  ];
  readonly key = (r: CustomerOption) => r.id;
  readonly display = (r: CustomerOption) => r.code + " · " + r.name;
  value() {
    return String(this.context().value || "") || null;
  }
  caption() {
    return String(this.context().row["customerName"] || "");
  }
}
@Component({
  selector: "app-crm-notes-field",
  imports: [AtlasTextarea],
  template: `<atlas-textarea
    [controlId]="context().controlId"
    [ariaLabel]="context().field.label"
    [value]="text()"
    (valueChange)="context().change($event)"
    [disabled]="context().disabled"
    [maxLength]="2000"
    [describedBy]="context().errorId"
  />`,
})
export class CrmNotesField {
  readonly context = input.required<FieldRendererContext>();
  text() {
    return String(this.context().value || "");
  }
}
@Component({
  selector: "app-crm-choice-field",
  imports: [AtlasRadioGroup],
  template: `<atlas-radio-group
    [controlId]="context().controlId"
    [label]="context().field.label"
    [options]="options()"
    [value]="text()"
    (valueChange)="context().change($event)"
    [disabled]="context().disabled"
    [describedBy]="context().errorId"
  />`,
})
export class CrmChoiceField {
  readonly context = input.required<FieldRendererContext>();
  text() {
    return String(this.context().value || "");
  }
  options() {
    return (this.context().field.options || []).map((value) => ({
      value,
      label: value,
    }));
  }
}
@Component({
  selector: "app-crm-active-field",
  imports: [AtlasToggle],
  template: `<atlas-toggle
    [controlId]="context().controlId"
    label="Available for new business"
    [checked]="checked()"
    (checkedChange)="context().change($event)"
    [disabled]="context().disabled"
  />`,
})
export class CrmActiveField {
  readonly context = input.required<FieldRendererContext>();
  checked() {
    return this.context().value === true;
  }
}
@Component({
  selector: "app-crm-date-field",
  imports: [AtlasDateInput],
  template: `<atlas-date-input
    [controlId]="context().controlId"
    [label]="context().field.label"
    [value]="text()"
    (valueChange)="context().change($event)"
    [disabled]="context().disabled"
    [describedBy]="context().errorId"
  />`,
})
export class CrmDateField {
  readonly context = input.required<FieldRendererContext>();
  text() {
    return String(this.context().value || "");
  }
}
