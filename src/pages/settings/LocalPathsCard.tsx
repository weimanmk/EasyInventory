import { Card, Descriptions } from 'antd';
import type { AppStatusDto } from '../../shared/types';

type LocalPathsCardProps = {
  appStatus?: AppStatusDto;
};

export function LocalPathsCard({ appStatus }: LocalPathsCardProps) {
  return (
    <Card title="本地路径">
      <Descriptions column={1} size="small">
        <Descriptions.Item label="数据库">{appStatus?.databasePath}</Descriptions.Item>
        <Descriptions.Item label="数据目录">{appStatus?.dataDir}</Descriptions.Item>
        <Descriptions.Item label="订单目录">{appStatus?.ordersDir}</Descriptions.Item>
        <Descriptions.Item label="导出目录">{appStatus?.exportsDir}</Descriptions.Item>
        <Descriptions.Item label="备份目录">{appStatus?.backupsDir}</Descriptions.Item>
        <Descriptions.Item label="日志目录">{appStatus?.logsDir}</Descriptions.Item>
        <Descriptions.Item label="版本">{appStatus?.version}</Descriptions.Item>
      </Descriptions>
    </Card>
  );
}
