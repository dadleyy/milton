'use strict';

const EmberApp = require('ember-cli/lib/broccoli/ember-app');
const autoprefixer = require('autoprefixer');
const tailwindcss = require('tailwindcss');

module.exports = function (defaults) {
  let app = new EmberApp(defaults, {
    postcssOptions: {
      compile: {
        enable: true,
        plugins: [
          {
            module: tailwindcss,
            options: {
              content: ['./app/**/*.hbs'],
              theme: {
                screens: {
                  lg: '960px',
                },
              },
            },
          },
          {
            module: autoprefixer,
          },
        ],
      },
    },
  });

  return app.toTree();
};
