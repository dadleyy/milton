import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';
import Session from 'octoprint-blinkrs/services/session';
import debugLogger from 'ember-debug-logger';

const debug = debugLogger('route:login');

class Login extends Route {
  @service
  public session!: Session;

  public async beforeModel(): Promise<void> {
    const { session } = this;
    const maybeSession = await session.current();

    if (maybeSession.getOrElse(undefined) !== undefined) {
      debug('user already logged in, sending to home');
      this.transitionTo('home');
      return;
    }
  }
}

export default Login;
