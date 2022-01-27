import Service from '@ember/service';
import config from 'octoprint-blinkrs/config/environment';

class Obelisk extends Service {
  public async query(): Promise<void> {
    await fetch(`${config.apiURL}control`);
  }
}

export default Obelisk;
