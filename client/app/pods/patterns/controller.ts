import Controller from '@ember/controller';
import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import { tracked } from '@glimmer/tracking';
import * as State from 'milton/pods/components/pattern-builder/state';
import MiltonAPI from 'milton/services/milton-api';
import debugLogger from 'ember-debug-logger';

const debug = debugLogger('controller:patterns');

class PatternController extends Controller {
  @service
  public miltonAPI!: MiltonAPI;

  @tracked
  public currentPatternState: State.State = State.empty();

  @tracked
  public busy: boolean = false;

  public get hasFrames(): boolean {
    return State.hasFrames(this.currentPatternState);
  }

  public get disabled(): boolean {
    const { hasFrames, busy } = this;
    return hasFrames === false || busy === true;
  }

  @action
  public async submit(): Promise<void> {
    const { miltonAPI, currentPatternState } = this;
    this.busy = true;
    debug('submitting pattern "%j"', currentPatternState);
    await miltonAPI.writePattern(currentPatternState);
    this.busy = false;
  }
}

export default PatternController;
