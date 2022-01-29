import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';
import Session from 'octoprint-blinkrs/services/session';
import Obelisk from 'octoprint-blinkrs/services/obelisk';
import * as Seidr from 'seidr';
import config from 'octoprint-blinkrs/config/environment';
import * as State from 'octoprint-blinkrs/pods/printer/state';
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

  public async model(): Promise<Seidr.Result<Error, State.State>> {
    const { obelisk } = this;
    debug('loading home model');
    const statusResult = await obelisk.query();
    const { snapshotURL } = config.apiConfig;
    return statusResult.map(status => ({ status, snapshotURL }));
  }
}

export default Home;
