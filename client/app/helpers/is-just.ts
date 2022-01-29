import { helper } from '@ember/component/helper';
import * as Seidr from 'seidr';

function isJust<T>(params: [Seidr.Maybe<T>]): boolean {
  const [first] = params;
  return first && first.map(() => true).getOrElse(false);
}

export default helper(isJust);
