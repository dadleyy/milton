import Service from '@ember/service';
import * as Seidr from 'seidr';
import config from 'milton/config/environment';
const { apiConfig } = config;

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
