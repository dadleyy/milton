import Component from '@glimmer/component';
import SumType, { Variants } from 'sums-up';

class CaseOf<T extends Variants> extends Component {
  public tagName = '';
  public kind!: SumType<T>;
}

export default CaseOf;
