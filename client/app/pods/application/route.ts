import Route from '@ember/routing/route';
import debugLogger from 'ember-debug-logger';
import config from 'milton/config/environment';

const debug = debugLogger('route:application');

class ApplicationRoute extends Route {
  public beforeModel(): void {
    debug('application route booting');
  }

  public model(): { version: string } {
    const version = (config.version || '').slice(0, 7);
    return { version };
  }
}

export default ApplicationRoute;
