'use strict';

module.exports = function (environment) {
  let ENV = {
    modulePrefix: 'milton',
    podModulePrefix: 'milton/pods',
    environment,
    rootURL: '/',
    locationType: 'history',

    apiConfig: {
      rootURL: '/',
      loginURL: 'http://127.0.0.1:8081/auth/start',
      snapshotURL: 'http://127.0.0.1:8081/control/snapshot',
    },

    EmberENV: {
      FEATURES: {
        // Here you can enable experimental features on an ember canary build
        // e.g. EMBER_NATIVE_DECORATOR_SUPPORT: true
      },
      EXTEND_PROTOTYPES: {
        // Prevent Ember Data from overriding Date.parse.
        Date: false,
      },
    },

    APP: {
      // Here you can pass flags/options to your application instance
      // when it is created
    },
  };

  if (environment === 'test') {
    // Testem prefers this...
    ENV.locationType = 'none';

    // keep test console output quieter
    ENV.APP.LOG_ACTIVE_GENERATION = false;
    ENV.APP.LOG_VIEW_LOOKUPS = false;

    ENV.APP.rootElement = '#ember-testing';
    ENV.APP.autoboot = false;
  }

  if (environment === 'production') {
    ENV.rootURL = process.env['OCTOPRINT_BLINKRS_UI_ROOT'] || ENV.rootURL;
    ENV.apiConfig.rootURL =
      process.env['OCTOPRINT_BLINKRS_API_ROOT'] || ENV.apiConfig.rootURL;
    ENV.apiConfig.loginURL =
      process.env['OCTOPRINT_BLINKRS_LOGIN_URL'] || ENV.apiConfig.loginURL;
    ENV.apiConfig.snapshotURL =
      process.env['OCTOPRINT_BLINKRS_SNAPSHOT_URL'] ||
      ENV.apiConfig.snapshotURL;
  }

  return ENV;
};
