import { catalogApi } from './catalog';
import { orderApi } from './orders';
import { reportApi } from './reports';
import { settingsApi } from './settings';
import { systemApi } from './system';

export const api = {
  ...systemApi,
  ...catalogApi,
  ...orderApi,
  ...reportApi,
  ...settingsApi
};
