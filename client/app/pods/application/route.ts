import Route from '@ember/routing/route';
import debugLogger from 'ember-debug-logger';

const debug = debugLogger('route:application');

class ApplicationRoute extends Route {
  public beforeModel(): void {
    debug('application route booting');
  }
}

export default ApplicationRoute;
