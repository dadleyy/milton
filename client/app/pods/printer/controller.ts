import Controller from '@ember/controller';
import { inject as service } from '@ember/service';
import { action } from '@ember/object';
import debugLogger from 'ember-debug-logger';
import MiltonAPI from 'milton/services/milton-api';

const debug = debugLogger('controller:printer');

class PrinterController extends Controller {
  @service
  public miltonApi!: MiltonAPI;

  @action
  public async refreshSnapshot(element: HTMLImageElement): Promise<void> {
    const { src } = element;
    debug('starting snapshot polling on element %o', element);

    while (!this.isDestroyed) {
      const ts = new Date();
      const id = btoa(ts.getTime() + '');
      element.src = `${src}?t=${id}`;
      await new Promise(resolve => setTimeout(resolve, 1000));
    }
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
}

export default PrinterController;
