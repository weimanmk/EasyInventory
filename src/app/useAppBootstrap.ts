import { message } from 'antd';
import { useEffect } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { api } from '../api/inventory';
import { useAppStore } from '../store/appStore';

export function useAppBootstrap() {
  const location = useLocation();
  const navigate = useNavigate();
  const {
    setupStatus,
    setStatus,
    setSetupStatus,
    setMerchant,
    setTerms,
    setFeatures,
    setProducts,
    setCustomers
  } = useAppStore();

  useEffect(() => {
    async function boot() {
      try {
        const [appStatus, products, customers, nextSetupStatus, nextMerchant, nextTerms, nextFeatures] = await Promise.all([
          api.status(),
          api.products({ isActive: true }),
          api.customers({ isActive: true }),
          api.setupStatus(),
          api.merchantProfile(),
          api.termSettings(),
          api.featureFlags()
        ]);
        setStatus(appStatus);
        setProducts(products);
        setCustomers(customers);
        setSetupStatus(nextSetupStatus);
        setMerchant(nextMerchant);
        setTerms(nextTerms);
        setFeatures(nextFeatures);
      } catch (error) {
        message.error(error instanceof Error ? error.message : '初始化失败');
      }
    }

    void boot();
  }, [setCustomers, setFeatures, setMerchant, setProducts, setSetupStatus, setStatus, setTerms]);

  useEffect(() => {
    if (setupStatus?.completed === false && location.pathname !== '/setup') {
      navigate('/setup', { replace: true });
    }
  }, [location.pathname, navigate, setupStatus?.completed]);
}
