// Types for compiled templates
declare module 'octoprint-blinkrs/*/template' {
  import { TemplateFactory } from 'htmlbars-inline-precompile';
  const tmpl: TemplateFactory;
  export default tmpl;
}
