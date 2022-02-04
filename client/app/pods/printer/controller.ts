import Controller from '@ember/controller';
import { inject as service } from '@ember/service';
import { action } from '@ember/object';
import { later, cancel } from '@ember/runloop';
import debugLogger from 'ember-debug-logger';
import MiltonAPI from 'milton/services/milton-api';

const debug = debugLogger('controller:printer');

class PrinterController extends Controller {
  @service
  public miltonApi!: MiltonAPI;

  @action
  public async stopRefresh(): Promise<void> {
    debug('terminating refresh');
    // @ts-ignore
    cancel(this.refreshTimeout);
  }

  @action
  public async refreshSnapshot(element: HTMLImageElement): Promise<void> {
    this.poll(element, element.src);
  }

  @action
  public async toggleLight(state: boolean): Promise<void> {
    const { miltonApi } = this;
    debug('toggling light "%s"', state);
    const result = await miltonApi.toggleLight(state);

    result.caseOf({
      Err: error => debug('[warning] unable to toggle - %s', error),
      Ok: () => debug('successfully toggled light'),
    });
  }

  private poll(element: HTMLImageElement, originalSource: string): void {
    debug('starting snapshot polling on element %o', element);
    const ts = new Date();
    const id = btoa(ts.getTime() + '');
    element.src = `${originalSource}?t=${id}`;
    // @ts-ignore
    this.refreshTimeout = later(() => this.poll(element, originalSource), 1000);
  }
}

export default PrinterController;
