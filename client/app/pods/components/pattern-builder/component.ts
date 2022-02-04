import Component from '@glimmer/component';
import { action } from '@ember/object';
import * as State from 'milton/pods/components/pattern-builder/state';
import debugLogger from 'ember-debug-logger';

const debug = debugLogger('component:pattern-builder');

type PatternBuilderArgs = {
  ledr: [number, number];
  state: State.State;
  onChange: (state: State.State) => void;
};

class PatternBuilder extends Component<PatternBuilderArgs> {
  public get state(): State.State {
    return this.args.state;
  }

  @action
  public setColor(location: [number, number], event: InputEvent): void {
    const { state } = this;
    const target = event.target as HTMLInputElement;
    const value = target.value;
    const [fi, ledn] = location;
    debug('setting frame[%s] led[%s] - "%s"', fi, ledn, value);
    this.args.onChange(State.setColor(state, fi, { ledn, hex: value }));
  }

  @action
  public addFrame(): void {
    const { state, args } = this;
    debug('adding new frame');
    const next = State.addFrame(state, args.ledr || [2, 4])
    this.args.onChange(next);
  }
}

export default PatternBuilder;
