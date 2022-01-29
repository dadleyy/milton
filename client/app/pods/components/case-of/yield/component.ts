import Component from '@glimmer/component';

class Yield<T> extends Component {
  public tagName = '';
  public data!: Array<T>;
}

export default Yield;
