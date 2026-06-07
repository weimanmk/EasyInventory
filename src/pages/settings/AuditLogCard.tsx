import { Card, Table } from 'antd';
import type { AuditLogDto } from '../../shared/types';
import { statusTag } from './statusTag';

type AuditLogCardProps = {
  auditLogs: AuditLogDto[];
};

export function AuditLogCard({ auditLogs }: AuditLogCardProps) {
  return (
    <Card title="审计日志">
      <Table
        rowKey="id"
        size="small"
        dataSource={auditLogs}
        columns={[
          { title: '时间', dataIndex: 'logTime', width: 170 },
          { title: '模块', dataIndex: 'module', width: 110 },
          { title: '动作', dataIndex: 'action', width: 140 },
          { title: '对象', dataIndex: 'targetLabel' },
          { title: '结果', render: (_, row) => statusTag(row.result), width: 90 },
          { title: '说明', dataIndex: 'message' }
        ]}
      />
    </Card>
  );
}
