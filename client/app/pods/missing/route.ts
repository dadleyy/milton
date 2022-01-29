import Route from '@ember/routing/route';

class MissingRoute extends Route {
  public beforeModel(): void {
    this.transitionTo('printer');
  }
}

export default MissingRoute;
