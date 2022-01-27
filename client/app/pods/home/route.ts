import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';
import Session from 'octoprint-blinkrs/services/session';
import Obelisk from 'octoprint-blinkrs/services/obelisk';
import debugLogger from 'ember-debug-logger';

const debug = debugLogger('route:home');

class Home extends Route {
  @service
  public session!: Session;

  @service
  public obelisk!: Obelisk;

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
    const { obelisk } = this;
    debug('loading home model');
    await obelisk.query();
  }
}

export default Home;
