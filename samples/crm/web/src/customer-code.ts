import {ChangeDetectionStrategy,Component,input} from '@angular/core';
import {AtlasInput} from '@bqatlas/ui';
import type {FieldRendererContext,FieldValidator} from '@bqatlas/angular';

export const validateCustomerCode: FieldValidator = value =>
  typeof value === 'string' && /^[A-Za-z0-9-]{1,32}$/.test(value.trim()) ? [] : ['Use 1–32 letters, digits or hyphens.'];

@Component({
  selector:'app-customer-code', imports:[AtlasInput], changeDetection:ChangeDetectionStrategy.OnPush,
  template:`<input atlasInput [id]="context().controlId" [value]="context().value ?? ''"
    [disabled]="context().disabled" [attr.aria-invalid]="context().errors.length > 0"
    [attr.aria-describedby]="context().errorId + ' ' + context().controlId + '-hint'"
    (input)="context().change($any($event.target).value)" />
    <small [id]="context().controlId + '-hint'">Letters, digits and hyphens. Codes are saved in uppercase.</small>`,
})
export class CustomerCodeField { readonly context=input.required<FieldRendererContext>(); }
