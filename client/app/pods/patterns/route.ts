import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';
import MiltonAPI from 'milton/services/milton-api';
import Session from 'milton/services/session';
import debugLogger from 'ember-debug-logger';

const debug = debugLogger('route:home');

class Patterns extends Route {
  @service
  public miltonApi!: MiltonAPI;

  @service
  public session!: Session;

  public async beforeModel(): Promise<void> {
    const { session } = this;
    const maybeSession = await session.current();
    const userInfo = maybeSession.getOrElse(undefined);

    if (!userInfo) {
      debug('no user info ready, redirecting to login');
      this.transitionTo('login');
      return;
    }
  }

  public async model(): Promise<void> {
    debug('loading patterns model');
  }
}

export default Patterns;
