import Route from '@ember/routing/route';

class MissingRoute extends Route {
  public beforeModel(): void {
    this.transitionTo('home');
  }
}

export default MissingRoute;
