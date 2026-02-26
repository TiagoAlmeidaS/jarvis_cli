// Dashboard JavaScript for Jarvis Daemon

// Get API key from localStorage or prompt
function getApiKey() {
    let apiKey = localStorage.getItem('jarvis_api_key');
    if (!apiKey) {
        apiKey = prompt('Digite sua API Key do Jarvis:');
        if (apiKey) {
            localStorage.setItem('jarvis_api_key', apiKey);
        }
    }
    return apiKey;
}

// API request helper
async function apiRequest(endpoint, options = {}) {
    const apiKey = getApiKey();
    if (!apiKey) {
        throw new Error('API Key não fornecida');
    }

    const response = await fetch(`/api${endpoint}`, {
        ...options,
        headers: {
            'Authorization': `Bearer ${apiKey}`,
            'Content-Type': 'application/json',
            ...options.headers,
        },
    });

    if (!response.ok) {
        if (response.status === 401) {
            localStorage.removeItem('jarvis_api_key');
            throw new Error('API Key inválida. Por favor, recarregue a página.');
        }
        throw new Error(`HTTP ${response.status}: ${await response.text()}`);
    }

    return response.json();
}

function dashboard() {
    return {
        activeTab: 'pipelines',
        healthStatus: 'UNKNOWN',
        pipelinesTotal: 0,
        pipelinesEnabled: 0,
        jobsRunning: 0,
        revenue: 0,
        goalsAtRisk: 0,
        proposalsPending: 0,
        pipelines: [],
        jobs: [],
        goals: [],
        proposals: [],
        logs: [],
        revenueChart: null,
        jobsChart: null,
        ws: null,

        async init() {
            await this.loadDashboard();
            this.setupCharts();
            this.connectWebSocket();
            // Auto-refresh every 30 seconds
            setInterval(() => this.loadDashboard(), 30000);
        },

        async loadDashboard() {
            try {
                const data = await apiRequest('/daemon/dashboard?days=30');
                this.healthStatus = data.health;
                this.pipelinesTotal = data.pipelines.length;
                this.pipelinesEnabled = data.pipelines.filter(p => p.enabled).length;
                // Get running jobs from status endpoint
                const statusData = await apiRequest('/daemon/status');
                this.jobsRunning = statusData.jobs.running;
                this.revenue = data.metrics.revenue.total_usd;
                this.goalsAtRisk = data.goals.filter(g => g.progress_pct < 40).length;
                this.proposalsPending = data.proposals ? data.proposals.length : 0;
                
                this.pipelines = data.pipelines;
                this.goals = data.goals;
                this.proposals = data.proposals || [];
                
                // Load jobs separately to get running count
                await this.loadJobs();
                
                // Get running jobs count from status
                try {
                    const statusData = await apiRequest('/daemon/status');
                    this.jobsRunning = statusData.jobs.running;
                } catch (error) {
                    console.error('Error loading status:', error);
                }
                
                // Load logs separately
                await this.loadLogs();
                
                // Update charts
                this.updateCharts(data);
            } catch (error) {
                console.error('Error loading dashboard:', error);
                alert('Erro ao carregar dashboard: ' + error.message);
            }
        },

        async loadJobs() {
            try {
                const data = await apiRequest('/daemon/jobs?limit=20');
                this.jobs = data.jobs;
            } catch (error) {
                console.error('Error loading jobs:', error);
            }
        },

        async loadLogs() {
            try {
                const data = await apiRequest('/daemon/logs?limit=50');
                this.logs = data.logs.reverse(); // Most recent first
                // Auto-scroll logs container
                const container = document.getElementById('logsContainer');
                if (container) {
                    container.scrollTop = container.scrollHeight;
                }
            } catch (error) {
                console.error('Error loading logs:', error);
            }
        },

        setupCharts() {
            // Revenue chart
            const revenueCtx = document.getElementById('revenueChart');
            if (revenueCtx) {
                this.revenueChart = new Chart(revenueCtx, {
                    type: 'line',
                    data: {
                        labels: [],
                        datasets: [{
                            label: 'Revenue (USD)',
                            data: [],
                            borderColor: 'rgb(34, 197, 94)',
                            backgroundColor: 'rgba(34, 197, 94, 0.1)',
                            tension: 0.4
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: {
                            legend: {
                                labels: { color: '#e5e7eb' }
                            }
                        },
                        scales: {
                            y: {
                                ticks: { color: '#9ca3af' },
                                grid: { color: '#374151' }
                            },
                            x: {
                                ticks: { color: '#9ca3af' },
                                grid: { color: '#374151' }
                            }
                        }
                    }
                });
            }

            // Jobs chart
            const jobsCtx = document.getElementById('jobsChart');
            if (jobsCtx) {
                this.jobsChart = new Chart(jobsCtx, {
                    type: 'doughnut',
                    data: {
                        labels: [],
                        datasets: [{
                            data: [],
                            backgroundColor: [
                                'rgba(59, 130, 246, 0.8)',
                                'rgba(34, 197, 94, 0.8)',
                                'rgba(239, 68, 68, 0.8)',
                                'rgba(156, 163, 175, 0.8)',
                            ]
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: {
                            legend: {
                                position: 'bottom',
                                labels: { color: '#e5e7eb' }
                            }
                        }
                    }
                });
            }
        },

        updateCharts(data) {
            // Update revenue chart (simplified - would need time series data)
            if (this.revenueChart) {
                this.revenueChart.data.labels = ['Hoje'];
                this.revenueChart.data.datasets[0].data = [data.metrics.revenue.total_usd];
                this.revenueChart.update();
            }

            // Update jobs chart
            if (this.jobsChart && this.jobs.length > 0) {
                const completed = data.jobs_24h.completed;
                const failed = data.jobs_24h.failed;
                const running = this.jobs.filter(j => j.status === 'running').length;
                const pending = this.jobs.filter(j => j.status === 'pending').length;
                
                this.jobsChart.data.labels = ['Running', 'Completed', 'Failed', 'Pending'];
                this.jobsChart.data.datasets[0].data = [running, completed, failed, pending];
                this.jobsChart.update();
            }
        },

        connectWebSocket() {
            const apiKey = getApiKey();
            if (!apiKey) return;

            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const wsUrl = `${protocol}//${window.location.host}/ws/daemon?token=${apiKey}`;
            
            this.ws = new WebSocket(wsUrl);
            
            this.ws.onopen = () => {
                console.log('WebSocket connected');
            };
            
            this.ws.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    if (data.event_type === 'status_update') {
                        // Update dashboard with real-time data
                        this.updateFromWebSocket(data.data);
                    }
                } catch (error) {
                    console.error('Error parsing WebSocket message:', error);
                }
            };
            
            this.ws.onerror = (error) => {
                console.error('WebSocket error:', error);
            };
            
            this.ws.onclose = () => {
                console.log('WebSocket disconnected, reconnecting...');
                setTimeout(() => this.connectWebSocket(), 5000);
            };
        },

        updateFromWebSocket(data) {
            if (data.pipelines) {
                this.pipelinesTotal = data.pipelines.total;
                this.pipelinesEnabled = data.pipelines.enabled;
            }
            if (data.jobs) {
                this.jobsRunning = data.jobs.running;
            }
            if (data.revenue) {
                this.revenue = data.revenue.total_usd_30d;
            }
            if (data.proposals) {
                this.proposalsPending = data.proposals.pending;
            }
        },

        async refresh() {
            await this.loadDashboard();
        },

        async togglePipeline(id, enabled) {
            try {
                const endpoint = enabled ? `/daemon/pipelines/${id}/enable` : `/daemon/pipelines/${id}/disable`;
                await apiRequest(endpoint, { method: 'POST' });
                await this.loadDashboard();
            } catch (error) {
                alert('Erro ao atualizar pipeline: ' + error.message);
            }
        },

        async approveProposal(id) {
            try {
                await apiRequest(`/daemon/proposals/${id}/approve`, { method: 'POST' });
                await this.loadDashboard();
            } catch (error) {
                alert('Erro ao aprovar proposta: ' + error.message);
            }
        },

        async rejectProposal(id) {
            try {
                await apiRequest(`/daemon/proposals/${id}/reject`, { method: 'POST' });
                await this.loadDashboard();
            } catch (error) {
                alert('Erro ao rejeitar proposta: ' + error.message);
            }
        },

        getStatusColor(status) {
            const colors = {
                'running': 'text-yellow-400',
                'completed': 'text-green-400',
                'failed': 'text-red-400',
                'pending': 'text-blue-400',
                'cancelled': 'text-gray-400',
            };
            return colors[status.toLowerCase()] || 'text-gray-400';
        },

        getRiskColor(risk) {
            const colors = {
                'high': 'text-red-400',
                'medium': 'text-yellow-400',
                'low': 'text-green-400',
            };
            return colors[risk.toLowerCase()] || 'text-gray-400';
        },

        getLogLevelColor(level) {
            const colors = {
                'error': 'text-red-400',
                'warn': 'text-yellow-400',
                'info': 'text-blue-400',
                'debug': 'text-gray-400',
            };
            return colors[level.toLowerCase()] || 'text-gray-400';
        },

        formatDate(timestamp) {
            if (!timestamp) return '-';
            const date = new Date(timestamp * 1000);
            return date.toLocaleString('pt-BR');
        },
    };
}
