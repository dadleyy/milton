import Route from '@ember/routing/route';

class IndexRoute extends Route {
  public beforeModel(): void {
    this.transitionTo('home');
  }
}

export default IndexRoute;
