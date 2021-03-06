import EmberRouter from '@ember/routing/router';
import config from 'milton/config/environment';

export default class Router extends EmberRouter {
  location = config.locationType;
  rootURL = config.rootURL;
}

Router.map(function () {
  this.route('printer');
  this.route('patterns');
  this.route('login');
  this.route('missing', { path: '*' });
});
