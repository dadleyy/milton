import Route from '@ember/routing/route';
import debugLogger from 'ember-debug-logger';
import config from 'milton/config/environment';

const debug = debugLogger('route:application');

class ApplicationRoute extends Route {
  public beforeModel(): void {
    debug('application route booting');
  }

  public model(): { version: string } {
    return { version: config.version };
  }
}

export default ApplicationRoute;
