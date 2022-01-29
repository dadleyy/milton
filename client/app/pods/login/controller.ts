import Controller from '@ember/controller';
import config from 'milton/config/environment';
const { apiConfig } = config;

class LoginController extends Controller {
  public loginURL: string = apiConfig.loginURL;
}

export default LoginController;
