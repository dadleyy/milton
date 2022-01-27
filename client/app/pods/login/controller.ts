import Controller from '@ember/controller';
import config from 'octoprint-blinkrs/config/environment';

class LoginController extends Controller {
  public loginURL: string = config.loginURL;
}

export default LoginController;
