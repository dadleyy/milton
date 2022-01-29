import { helper } from '@ember/component/helper';
import { camelize } from '@ember/string';

export default helper(([input]: [string]) => camelize(input));
