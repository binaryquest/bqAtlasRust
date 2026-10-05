import { ChangeDetectionStrategy, Component, signal } from "@angular/core";
import { FormsModule } from "@angular/forms";
import {
  AtlasTab,
  AtlasTabs,
  AtlasFieldset,
  AtlasAccordionSection,
  AtlasSplitPane,
  AtlasPanel,
  AtlasInput,
  AtlasButton,
} from "@bqatlas/ui";
@Component({
  selector: "demo-layouts",
  imports: [
    FormsModule,
    AtlasTab,
    AtlasTabs,
    AtlasFieldset,
    AtlasAccordionSection,
    AtlasSplitPane,
    AtlasPanel,
    AtlasInput,
    AtlasButton,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `<div class="lab-section-intro">
      <div>
        <h3>Layout workbench</h3>
        <p>
          Resize the separator with a pointer or arrow keys. Tabs and collapsed
          sections keep their form values.
        </p>
      </div>
    </div>
    <atlas-split-pane
      primaryLabel="Supplier form"
      secondaryLabel="Live summary"
      [ratio]="60"
      [maximum]="75"
      [breakpoint]="600"
    >
      <atlas-panel atlasSplitPrimary title="Supplier onboarding">
        <span panelActions class="showcase-badge">Draft example</span>
        <div panelToolbar class="demo-options">
          <button
            atlasButton
            (click)="
              name = 'Northstar Supply'; notes = ''; message.set('Form reset.')
            "
          >
            Reset form</button
          ><span role="status">{{ message() }}</span>
        </div>
        <atlas-tabs label="Supplier sections">
          <ng-template atlasTab="company" label="Company"
            ><atlas-fieldset label="Company information" [collapsible]="true"
              ><label class="customer-notes"
                >Supplier name<input
                  atlasInput
                  [(ngModel)]="name" /></label></atlas-fieldset
            ><atlas-accordion-section label="Internal notes" [expanded]="false"
              ><label class="customer-notes"
                >Supplier notes<textarea
                  atlasInput
                  rows="3"
                  [(ngModel)]="notes"
                ></textarea></label></atlas-accordion-section
          ></ng-template>
          <ng-template atlasTab="delivery" label="Delivery"
            ><atlas-fieldset label="Delivery preferences" [disabled]="true"
              ><label class="customer-notes"
                >Delivery address<input
                  atlasInput
                  value="Demonstration of a disabled fieldset" /></label
            ></atlas-fieldset>
            <p class="lab-note">
              A disabled fieldset disables its native descendant controls
              together.
            </p></ng-template
          >
          <ng-template atlasTab="locked" label="Unavailable" [disabled]="true"
            ><p>Disabled tab.</p></ng-template
          > </atlas-tabs
        ><span panelFooter>Header · toolbar · body · footer slots</span>
      </atlas-panel>
      <atlas-panel atlasSplitSecondary title="Live summary"
        ><h4>{{ name || "Unnamed supplier" }}</h4>
        <p>{{ notes || "Add notes in the Company tab." }}</p>
        <p class="lab-note">
          Summary stays connected while tabs switch and fieldsets collapse.
        </p></atlas-panel
      >
    </atlas-split-pane>`,
})
export class LayoutExamples {
  name = "Northstar Supply";
  notes = "";
  readonly message = signal("");
}
