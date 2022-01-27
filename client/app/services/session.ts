import Service from '@ember/service';
import { tracked } from '@glimmer/tracking';
import SumType from 'sums-up';
import * as Seidr from 'seidr';
import debugLogger from 'ember-debug-logger';
import config from 'octoprint-blinkrs/config/environment';

const debug = debugLogger('service:session');

type IdentifyUserInfo = {
  sub: string;
  picture: string;
  nickname: string;
  email: string;
};

type IdentifyResponse = {
  ok: boolean;
  timestamp: string;
  user?: IdentifyUserInfo;
};

class SessionState extends SumType<{ NotRequested: []; Available: [IdentifyUserInfo]; NotAvailable: []; }> {
};

function Available(user: IdentifyUserInfo): SessionState {
  return new SessionState('Available', user);
}

function NotAvailable(): SessionState {
  return new SessionState('NotAvailable');
}

function NotRequested(): SessionState {
  return new SessionState('NotRequested');
}

class Session extends Service {
  @tracked
  private _session: SessionState = NotRequested();

  public async current(): Promise<Seidr.Maybe<IdentifyUserInfo>> {
    const { _session: session } = this;

    return session.caseOf({
      NotRequested: async () => {
        debug('session not yet requested, attempting to load from "%s"', config.apiURL);

        try {
          const response = await fetch(`${config.apiURL}auth/identify`);

          if (response.status != 200) {
            debug('invalid response status code, skipping');
            this._session = NotAvailable();
            return Seidr.Nothing();
          }

          const payload: IdentifyResponse = await response.json();
          const user = Seidr.Maybe.fromNullable(payload.user);
          this._session = user.map(Available).getOrElse(NotAvailable());
          return user;
        } catch (error) {
          debug('unable to request session - %s', error);
        }

        this._session = NotAvailable();
        return Seidr.Nothing();
      },
      NotAvailable: () => Promise.resolve(Seidr.Nothing()),
      Available: info => Promise.resolve(Seidr.Just(info)),
    });
  }
}

export default Session;
