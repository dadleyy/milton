import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';
import Session from 'milton/services/session';
import MiltonAPI from 'milton/services/milton-api';
import * as Seidr from 'seidr';
import config from 'milton/config/environment';
import * as State from 'milton/pods/printer/state';
import debugLogger from 'ember-debug-logger';

const debug = debugLogger('route:home');

class Home extends Route {
  @service
  public session!: Session;

  @service
  public miltonApi!: MiltonAPI;

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
    const { miltonApi } = this;
    debug('loading home model');
    const statusResult = await miltonApi.query();
    const { snapshotURL } = config.apiConfig;
    return statusResult.map(status => ({ status, snapshotURL }));
  }
}

export default Home;
