import Route from '@ember/routing/route';

class IndexRoute extends Route {
  public beforeModel(): void {
    this.transitionTo('printer');
  }
}

export default IndexRoute;
