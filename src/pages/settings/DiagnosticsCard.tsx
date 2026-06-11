import { Button, Card, Descriptions, List, Space, Table } from 'antd';
import type {
  AppStatusDto,
  BackupDto,
  DataSelfCheckDto,
  DiagnosticSummaryDto
} from '../../shared/types';
import { statusTag } from './statusTag';

type DiagnosticsCardProps = {
  appStatus?: AppStatusDto;
  backups: BackupDto[];
  diagnostic?: DiagnosticSummaryDto;
  selfCheck?: DataSelfCheckDto;
  onRunSelfCheck: () => void;
  onExportSelfCheck: () => void;
  onExportDiagnosticPackage: () => void;
  onRefresh: () => void;
};

export function DiagnosticsCard({
  appStatus,
  backups,
  diagnostic,
  selfCheck,
  onRunSelfCheck,
  onExportSelfCheck,
  onExportDiagnosticPackage,
  onRefresh
}: DiagnosticsCardProps) {
  return (
    <Card title="诊断中心">
      <Space style={{ marginBottom: 12 }}>
        <Button type="primary" onClick={onRunSelfCheck}>运行数据自检</Button>
        <Button onClick={onExportSelfCheck}>导出自检结果</Button>
        <Button onClick={onExportDiagnosticPackage}>导出诊断包</Button>
        <Button onClick={onRefresh}>刷新诊断信息</Button>
      </Space>
      <Descriptions column={2} size="small">
        <Descriptions.Item label="数据库">{diagnostic?.databasePath ?? finalPathComponent(appStatus?.databasePath)}</Descriptions.Item>
        <Descriptions.Item label="数据库大小">{diagnostic?.databaseSize ?? 0} B</Descriptions.Item>
        <Descriptions.Item label="版本">{diagnostic?.version ?? appStatus?.version}</Descriptions.Item>
        <Descriptions.Item label="备份数量">{diagnostic?.backupCount ?? backups.length}</Descriptions.Item>
        <Descriptions.Item label="最近备份">{diagnostic?.latestBackupAt ?? '-'}</Descriptions.Item>
        <Descriptions.Item label="基础统计">
          商品 {diagnostic?.productCount ?? 0} / 客户 {diagnostic?.customerCount ?? 0} / 订单 {diagnostic?.orderCount ?? 0} / 单据 {diagnostic?.documentCount ?? 0}
        </Descriptions.Item>
      </Descriptions>
      {selfCheck && (
        <div style={{ marginTop: 12 }}>
          <Descriptions column={4} size="small">
            <Descriptions.Item label="自检时间">{selfCheck.checkedAt}</Descriptions.Item>
            <Descriptions.Item label="异常数">{selfCheck.issueCount}</Descriptions.Item>
            <Descriptions.Item label="库存">{selfCheck.inventoryChecked}</Descriptions.Item>
            <Descriptions.Item label="订单">{selfCheck.ordersChecked}</Descriptions.Item>
            <Descriptions.Item label="月费">{selfCheck.creditsChecked}</Descriptions.Item>
            <Descriptions.Item label="单据">{selfCheck.documentsChecked}</Descriptions.Item>
          </Descriptions>
          <Table
            rowKey={(row) => `${row.checkCode}-${row.targetType}-${row.targetId ?? row.targetLabel}`}
            size="small"
            dataSource={selfCheck.issues}
            columns={[
              { title: '级别', render: (_, row) => statusTag(row.severity), width: 90 },
              { title: '检查项', dataIndex: 'checkCode', width: 170 },
              { title: '对象', dataIndex: 'targetLabel', width: 180 },
              { title: '说明', dataIndex: 'message' },
              { title: '详情', dataIndex: 'details' }
            ]}
          />
        </div>
      )}
      <Card size="small" title="最近日志" style={{ marginTop: 12 }}>
        <List
          size="small"
          dataSource={diagnostic?.latestLogs ?? []}
          renderItem={(item) => <List.Item>{item}</List.Item>}
        />
      </Card>
    </Card>
  );
}

function finalPathComponent(path?: string) {
  if (!path) {
    return undefined;
  }
  const normalized = path.replace(/\\/g, '/');
  const parts = normalized.split('/').filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : path;
}
