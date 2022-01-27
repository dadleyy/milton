import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';
import Obelisk from 'octoprint-blinkrs/services/obelisk';
import Session from 'octoprint-blinkrs/services/session';
import debugLogger from 'ember-debug-logger';

const debug = debugLogger('route:home');

class Patterns extends Route {
  @service
  public obelisk!: Obelisk;

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
