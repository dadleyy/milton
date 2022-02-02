import Service from '@ember/service';
import * as Seidr from 'seidr';
import config from 'milton/config/environment';
import debugLogger from 'ember-debug-logger';

const debug = debugLogger('service:milton-api');
const { apiConfig } = config;

const POST_CONFIG = {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
};

export type OctoprintStatus = {
  job: {
    file: {
      name: string;
    };
  };
  progress: {
    completion: number;
  };
  state: string;
};

class MiltonAPI extends Service {
  public async toggleLight(state: boolean): Promise<Seidr.Result<Error, boolean>> {
    debug('sending control request - "%s"', state);

    try {
      const parameters = { ...POST_CONFIG, body: JSON.stringify({ mode: state ? 'on' : 'off' }) };
      const response = await fetch(`${apiConfig.rootURL}control`, parameters);
      debug('control response status "%s"', response.status);

      if (response.status !== 200) {
        return Seidr.Err(new Error(`bad response - ${response.status}`));
      }

      return Seidr.Ok(state);
    } catch (error) {
      return Seidr.Err(error);
    }
  }

  public async query(): Promise<Seidr.Result<Error, OctoprintStatus>> {
    try {
      const response = await fetch(`${apiConfig.rootURL}control`);
      return Seidr.Ok(await response.json());
    } catch (error) {
      return Seidr.Err(error);
    }
  }
}

export default MiltonAPI;
