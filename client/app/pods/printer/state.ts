import { OctoprintStatus } from 'octoprint-blinkrs/services/obelisk';

export type State = {
  status: OctoprintStatus;
  snapshotURL: string;
};
