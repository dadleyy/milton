import { OctoprintStatus } from 'milton/services/milton-api';

export type State = {
  status: OctoprintStatus;
  snapshotURL: string;
};
