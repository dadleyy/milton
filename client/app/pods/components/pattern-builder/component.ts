import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';
import { action } from '@ember/object';
import * as State from 'octoprint-blinkrs/pods/components/pattern-builder/state';
import debugLogger from 'ember-debug-logger';

const debug = debugLogger('component:pattern-builder');

type PatternBuilderArgs = {
  ledr: [number, number];
};

class PatternBuilder extends Component<PatternBuilderArgs> {
  @tracked
  public state: State.State = State.empty();

  @action
  public setColor(location: [number, number], event: InputEvent): void {
    const { state } = this;
    const target = event.target as HTMLInputElement;
    const value = target.value;
    const [fi, ledn] = location;
    debug('setting frame[%s] led[%s] - "%s"', fi, ledn, value);
    this.state = State.setColor(state, fi, { ledn, hex: value });
  }

  @action
  public addFrame(): void {
    const { state, args } = this;
    debug('adding new frame');
    this.state = State.addFrame(state, args.ledr || [2, 4])
  }
}

export default PatternBuilder;
