import { Button, Card, Space, Table } from 'antd';
import type { BackupDto } from '../../shared/types';
import { statusTag } from './statusTag';

type BackupRestoreCardProps = {
  backups: BackupDto[];
  onBackup: () => void;
  onOpenBackupFolder: () => void;
  onRestoreBackup: (backup: BackupDto) => void;
};

export function BackupRestoreCard({
  backups,
  onBackup,
  onOpenBackupFolder,
  onRestoreBackup
}: BackupRestoreCardProps) {
  return (
    <Card title="备份与恢复">
      <Space>
        <Button onClick={onBackup}>立即备份</Button>
        <Button onClick={onOpenBackupFolder}>打开备份目录</Button>
      </Space>
      <Table
        rowKey="id"
        size="small"
        style={{ marginTop: 12 }}
        dataSource={backups}
        columns={[
          { title: '时间', dataIndex: 'createdAt', width: 160 },
          { title: '类型', dataIndex: 'backupType', width: 120 },
          { title: '状态', render: (_, row) => statusTag(row.status), width: 100 },
          { title: '路径', dataIndex: 'backupPath' },
          {
            title: '操作',
            width: 100,
            render: (_, row) => (
              <Button size="small" danger disabled={row.status !== 'success'} onClick={() => onRestoreBackup(row)}>
                恢复
              </Button>
            )
          }
        ]}
      />
    </Card>
  );
}
