import { helper } from '@ember/component/helper';
import { underscore as camelize } from '@ember/string';

export default helper(([input]: [string]) => camelize(input));
