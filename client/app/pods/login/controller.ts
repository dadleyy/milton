import Controller from '@ember/controller';
import config from 'octoprint-blinkrs/config/environment';
const { apiConfig } = config;

class LoginController extends Controller {
  public loginURL: string = apiConfig.loginURL;
}

export default LoginController;
