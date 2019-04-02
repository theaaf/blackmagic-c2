import * as React from 'react';

import {Query, Mutation} from 'react-apollo';

import {AgentQuery as AgentQueryData} from '../../__generated__/graphql';
class AgentQuery extends Query<AgentQueryData> {}

import {HyperDeckCommandMutation as HyperDeckCommandMutationData} from '../../__generated__/graphql';
class HyperDeckCommandMutation extends Mutation<HyperDeckCommandMutationData> {}

import gql from 'graphql-tag';

import {
    Button, createStyles, Divider, FormControlLabel, Grid, IconButton, Menu, MenuItem, Modal, Paper,
    Switch, Table, TableBody, TableCell, TableHead, TableRow, TextField, Theme, Toolbar, Typography,
    withStyles, WithStyles
} from '@material-ui/core';
import {MoreVert as MoreVertIcon, OpenInNew as OpenInNewIcon} from '@material-ui/icons';

const styles = (theme: Theme) => createStyles({
    buttonIcon: {
        marginLeft: '6px',
    },
    grow: {
        flexGrow: 1,
    },
    hyperDeckCommandModal: {
        height: '60vh',
        left: '20vw',
        top: '20vh',
        width: '60vw',
    },
    hyperDeckCommandForm: {
        maxHeight: '100%',
        display: 'flex',
        flexDirection: 'column',
    },
    keyValueGrid: {
        margin: '16px',
    },
    modalRoot: {
        height: '100%',
        padding: '16px',
    },
    paper: {
        marginBottom: '32px',
        padding: '16px',
    },
    root: {
        padding: '16px',
    },
});

interface OverviewProps extends WithStyles<typeof styles> {
    id: string;
}

interface OverviewState {
    hyperDeckCommandInput: string;
    hyperDeckCommandModalOpen: boolean;
    networkDeviceMenuAnchor: HTMLElement | null;
    openNetworkDeviceMenu: string;
    showingAllNetworkDevices: boolean;
}

export const Overview = withStyles(styles)(
    class extends React.Component<OverviewProps, OverviewState> {
        constructor(props: any) {
            super(props);

            this.state = {
                hyperDeckCommandInput: '',
                hyperDeckCommandModalOpen: false,
                networkDeviceMenuAnchor: null,
                openNetworkDeviceMenu: '',
                showingAllNetworkDevices: false,
            };
        }

        renderKeyValue(label: string, value: null | string | number | boolean, options?: {hex: boolean}) {
            if (value === null) {
                return null;
            }
            let valueStr = '';
            if (typeof value === 'string') {
                valueStr = value;
            } else if (typeof value === 'number') {
                if (options && options.hex) {
                    valueStr = `0x${value.toString(16)}`;
                } else {
                    valueStr = value.toString();
                }
            } else if (typeof value === 'boolean') {
                valueStr = value ? 'Yes' : 'No';
            }
            return (
                <Grid item key={label} xs={3}>
                    <Typography><b>{label}:</b> {valueStr}</Typography>
                </Grid>
            );
        }

        busyString(busy: {
            playbackBusy: boolean;
            captureBusy: boolean;
            serialPortBusy: boolean;
        }): string {
            const parts = [];
            if (busy.playbackBusy) {
                parts.push('Playback');
            }
            if (busy.captureBusy) {
                parts.push('Capture');
            }
            if (busy.serialPortBusy) {
                parts.push('Serial Port');
            }
            return parts.length > 0 ? `Yes (${parts.join(' / ')})` : 'No';
        }

        closeNetworkDeviceMenu() {
            this.setState({
                networkDeviceMenuAnchor: null,
                openNetworkDeviceMenu: '',
            });
        }

        renderHyperDeckMenu(macAddress: string, ipAddress: string) {
            return (
                <React.Fragment>
                    <Menu
                        anchorEl={this.state.networkDeviceMenuAnchor}
                        open={this.state.openNetworkDeviceMenu === macAddress}
                        onClose={() => {
                            this.closeNetworkDeviceMenu();
                        }}
                    >
                        <MenuItem
                            onClick={() => {
                                this.setState({
                                    hyperDeckCommandInput: '',
                                    hyperDeckCommandModalOpen: true,
                                });
                                this.closeNetworkDeviceMenu();
                            }}
                        >
                            Execute Command
                        </MenuItem>
                    </Menu>
                    <Modal
                        className={this.props.classes.hyperDeckCommandModal}
                        open={this.state.hyperDeckCommandModalOpen}
                        onClose={() => {
                            this.setState({
                                hyperDeckCommandModalOpen: false,
                            });
                        }}
                    >
                        <Paper className={this.props.classes.modalRoot}>
                            <HyperDeckCommandMutation
                                mutation={gql`
                                    mutation HyperDeckCommand($agentId: String!, $ipAddress: String!, $command: String!) {
                                        hyperdeckCommand(agentId: $agentId, ipAddress: $ipAddress, command: $command) {
                                            code
                                            text
                                            payload
                                        }
                                    }
                                `}
                            >
                                {(execute, {data, error, loading}) => (
                                    <form
                                        className={this.props.classes.hyperDeckCommandForm}
                                        onSubmit={(e) => {
                                            execute({
                                                variables: {
                                                    agentId: this.props.id,
                                                    command: this.state.hyperDeckCommandInput,
                                                    ipAddress,
                                                },
                                            });
                                            e.preventDefault();
                                        }}
                                    >
                                        <TextField
                                            fullWidth={true}
                                            label="Command"
                                            onChange={(e) => {
                                                this.setState({
                                                    hyperDeckCommandInput: e.target.value,
                                                });
                                            }}
                                            value={this.state.hyperDeckCommandInput}
                                        />
                                        <div style={{overflow: 'scroll'}}>
                                            <pre><code>
                                                {loading && 'Loading...'}
                                                {error && 'Error: ' + error.message}
                                                {data && `${data.hyperdeckCommand.code} ${data.hyperdeckCommand.text}${(data.hyperdeckCommand.payload && ':\n' + data.hyperdeckCommand.payload) || ''}`}
                                            </code></pre>
                                        </div>
                                    </form>
                                )}
                            </HyperDeckCommandMutation>
                        </Paper>
                    </Modal>
                </React.Fragment>
            );
        }

        render() {
            return (
                <div className={this.props.classes.root}>
                    <AgentQuery
                        pollInterval={1000}
                        query={gql`
                            query Agent($id: String!) {
                                agent(id: $id) {
                                    id
                                    state {
                                        networkDevices {
                                            ipAddress
                                            macAddress
                                            details {
                                                ... on HyperDeckDetails {
                                                    modelName
                                                    uniqueId
                                                    protocolVersion
                                                }
                                            }
                                        }
                                        decklinkDevices {
                                            modelName
                                            attributes {
                                                supportsInternalKeying
                                                supportsExternalKeying
                                                supportsHdKeying
                                                supportsInputFormatDetection
                                                hasReferenceInput
                                                hasSerialPort
                                                hasAnalogVideoOutputGain
                                                canOnlyAdjustOverallVideoOutputGain
                                                hasVideoInputAntialiasingFilter
                                                hasBypass
                                                supportsClockTimingAdjustment
                                                supportsFullDuplex
                                                supportsFullFrameReferenceInputTimingOffset
                                                supportsSmpteLevelAOutput
                                                supportsDualLinkSdi
                                                supportsQuadLinkSdi
                                                supportsIdleOutput
                                                hasLtcTimecodeInput
                                                supportsDuplexModeConfiguration
                                                supportsHdrMetadata
                                                supportsColorspaceMetadata
                                                maximumAudioChannels
                                                maximumAnalogAudioInputChannels
                                                maximumAnalogAudioOutputChannels
                                                numberOfSubdevices
                                                subdeviceIndex
                                                persistentId
                                                deviceGroupId
                                                topologicalId
                                                videoIoSupport {
                                                    capture
                                                    playback
                                                }
                                                audioInputRcaChannelCount
                                                audioInputXlrChannelCount
                                                audioOutputRcaChannelCount
                                                audioOutputXlrChannelCount
                                                pairedDevicePersistentId
                                                videoInputGainMinimum
                                                videoInputGainMaximum
                                                videoOutputGainMinimum
                                                videoOutputGainMaximum
                                                microphoneInputGainMinimum
                                                microphoneInputGainMaximum
                                                serialPortDeviceName
                                                vendorName
                                                displayName
                                                modelName
                                                deviceHandle
                                            }
                                            status {
                                                videoInputSignalLocked
                                                referenceSignalLocked
                                                receivedEdid
                                                detectedVideoInputMode {
                                                    name
                                                }
                                                currentVideoInputMode {
                                                    name
                                                }
                                                currentVideoOutputMode {
                                                    name
                                                }
                                                pciExpressLinkWidth
                                                pciExpressLinkSpeed
                                                busy {
                                                    playbackBusy
                                                    captureBusy
                                                    serialPortBusy
                                                }
                                                deviceTemperature
                                            }
                                        }
                                    }
                                }
                            }
                        `}
                        variables={{
                            id: this.props.id,
                        }}
                    >
                        {({loading, error, data}) => {
                            if (loading && (!data || !data.agent)) {
                                return <Typography>Loading...</Typography>;
                            } else if (error) {
                                alert(error);
                                return null;
                            }
                            return data && data.agent && (
                                <React.Fragment>
                                    <Toolbar>
                                        <Typography className={this.props.classes.grow} variant="h6">{data.agent.id}</Typography>
                                        <Button
                                            color="primary"
                                            onClick={() => {
                                                window.open(window.location + '/shell');
                                            }}
                                        >
                                            Shell
                                            <OpenInNewIcon className={this.props.classes.buttonIcon} />
                                        </Button>
                                    </Toolbar>
                                    {data.agent.state.networkDevices.length > 0 && (
                                        <Paper className={this.props.classes.paper}>
                                            <Toolbar>
                                                <Typography className={this.props.classes.grow} variant="h6">Network Devices</Typography>
                                                <FormControlLabel
                                                    control={
                                                        <Switch
                                                            checked={this.state.showingAllNetworkDevices}
                                                            onChange={(e) => {
                                                                this.setState({
                                                                    showingAllNetworkDevices: e.target.checked,
                                                                });
                                                            }}
                                                            color="primary"
                                                        />
                                                    }
                                                    label="Show All"
                                                />
                                            </Toolbar>
                                            <Table>
                                                <TableHead>
                                                    <TableRow>
                                                        <TableCell>IP Address</TableCell>
                                                        <TableCell>MAC Address</TableCell>
                                                        <TableCell>Type</TableCell>
                                                        <TableCell>Description</TableCell>
                                                        <TableCell></TableCell>
                                                    </TableRow>
                                                </TableHead>
                                                <TableBody>
                                                    {data.agent.state.networkDevices.map((device) => {
                                                        if (!this.state.showingAllNetworkDevices && !device.details) {
                                                            return null;
                                                        }
                                                        const menuOpen = this.state.openNetworkDeviceMenu === device.macAddress;
                                                        let typeName = '';
                                                        let description = '';
                                                        let menu = null;
                                                        if (device.details) {
                                                            if (device.details.__typename === 'HyperDeckDetails') {
                                                                typeName = device.details.modelName;
                                                                description = `Protocol Version: ${device.details.protocolVersion}, Unique Id: ${device.details.uniqueId}`;
                                                                menu = this.renderHyperDeckMenu(device.macAddress, device.ipAddress);
                                                            }
                                                        }
                                                        return (
                                                            <TableRow key={device.macAddress}>
                                                                <TableCell>{device.ipAddress}</TableCell>
                                                                <TableCell>{device.macAddress}</TableCell>
                                                                <TableCell>{typeName}</TableCell>
                                                                <TableCell>{description}</TableCell>
                                                                <TableCell>
                                                                    {menu && (
                                                                        <IconButton
                                                                            onClick={(e) => {
                                                                                this.setState({
                                                                                    networkDeviceMenuAnchor: e.currentTarget,
                                                                                    openNetworkDeviceMenu: device.macAddress,
                                                                                });
                                                                            }}
                                                                        >
                                                                            <MoreVertIcon />
                                                                        </IconButton>
                                                                    )}
                                                                    {menu}
                                                                </TableCell>
                                                            </TableRow>
                                                        );
                                                    })}
                                                </TableBody>
                                            </Table>
                                        </Paper>
                                    )}
                                    {data.agent.state.decklinkDevices.map((device) => (
                                        <Paper className={this.props.classes.paper} key={device.attributes.deviceHandle || ''}>
                                            <Toolbar>
                                                <Typography variant="h6">{device.attributes.displayName || device.modelName}</Typography>
                                            </Toolbar>
                                            <Divider variant="middle" />
                                            <Grid className={this.props.classes.keyValueGrid} container spacing={16}>
                                                {this.renderKeyValue('Video Input Signal Locked', device.status.videoInputSignalLocked)}
                                                {this.renderKeyValue('Reference Signal Locked', device.status.referenceSignalLocked)}
                                                {this.renderKeyValue('Received EDID', device.status.receivedEdid)}
                                                {this.renderKeyValue('Detected Video Input Mode', device.status.detectedVideoInputMode && device.status.detectedVideoInputMode.name)}
                                                {this.renderKeyValue('Current Video Input Mode', device.status.currentVideoInputMode && device.status.currentVideoInputMode.name)}
                                                {this.renderKeyValue('Current Video Output Mode', device.status.currentVideoOutputMode && device.status.currentVideoOutputMode.name)}
                                                {this.renderKeyValue('PCI Express Link Width', device.status.pciExpressLinkWidth)}
                                                {this.renderKeyValue('PCI Express Link Speed', device.status.pciExpressLinkSpeed)}
                                                {this.renderKeyValue('Busy', device.status.busy ? this.busyString(device.status.busy) : null)}
                                                {this.renderKeyValue('Device Temperature', device.status.deviceTemperature)}
                                            </Grid>
                                            <Divider variant="middle" />
                                            <Grid className={this.props.classes.keyValueGrid} container spacing={16}>
                                                {this.renderKeyValue('Supports Internal Keying', device.attributes.supportsInternalKeying)}
                                                {this.renderKeyValue('Supports External Keying', device.attributes.supportsExternalKeying)}
                                                {this.renderKeyValue('Supports HD Keying', device.attributes.supportsHdKeying)}
                                                {this.renderKeyValue('Supports Input Format Detection', device.attributes.supportsInputFormatDetection)}
                                                {this.renderKeyValue('Has Reference Input', device.attributes.hasReferenceInput)}
                                                {this.renderKeyValue('Has Serial Port', device.attributes.hasSerialPort)}
                                                {this.renderKeyValue('Has Analog Video Output Gain', device.attributes.hasAnalogVideoOutputGain)}
                                                {this.renderKeyValue('Can Only Adjust Overall Video Output Gain', device.attributes.canOnlyAdjustOverallVideoOutputGain)}
                                                {this.renderKeyValue('Has Video Input Antialiasing Filter', device.attributes.hasVideoInputAntialiasingFilter)}
                                                {this.renderKeyValue('Has Bypass', device.attributes.hasBypass)}
                                                {this.renderKeyValue('Supports Clock Timing Adjustment', device.attributes.supportsClockTimingAdjustment)}
                                                {this.renderKeyValue('Supports Full Duplex', device.attributes.supportsFullDuplex)}
                                                {this.renderKeyValue('Supports Full Frame Reference Input Timing Offset', device.attributes.supportsFullFrameReferenceInputTimingOffset)}
                                                {this.renderKeyValue('Supports SMPTE Level A Output', device.attributes.supportsSmpteLevelAOutput)}
                                                {this.renderKeyValue('Supports Dual Link SDI', device.attributes.supportsDualLinkSdi)}
                                                {this.renderKeyValue('Supports Quad Link SDI', device.attributes.supportsQuadLinkSdi)}
                                                {this.renderKeyValue('Supports Idle Output', device.attributes.supportsIdleOutput)}
                                                {this.renderKeyValue('Has LTC Timecode Input', device.attributes.hasLtcTimecodeInput)}
                                                {this.renderKeyValue('Supports Duplex Mode Configuration', device.attributes.supportsDuplexModeConfiguration)}
                                                {this.renderKeyValue('Supports HDR Metadata', device.attributes.supportsHdrMetadata)}
                                                {this.renderKeyValue('Supports Colorspace Metadata', device.attributes.supportsColorspaceMetadata)}
                                                {this.renderKeyValue('Maximum Audio Channels', device.attributes.maximumAudioChannels)}
                                                {this.renderKeyValue('Maximum Analog Audio Input Channels', device.attributes.maximumAnalogAudioInputChannels)}
                                                {this.renderKeyValue('Maximum Analog Audio Output Channels', device.attributes.maximumAnalogAudioOutputChannels)}
                                                {this.renderKeyValue('Number Of Subdevices', device.attributes.numberOfSubdevices)}
                                                {this.renderKeyValue('Subdevice Index', device.attributes.subdeviceIndex)}
                                                {this.renderKeyValue('Persistent Id', device.attributes.persistentId, {hex: true})}
                                                {this.renderKeyValue('Device Group Id', device.attributes.deviceGroupId, {hex: true})}
                                                {this.renderKeyValue('Topological Id', device.attributes.topologicalId, {hex: true})}
                                                {this.renderKeyValue('Supports Capture', device.attributes.videoIoSupport && device.attributes.videoIoSupport.capture)}
                                                {this.renderKeyValue('Supports Playback', device.attributes.videoIoSupport && device.attributes.videoIoSupport.playback)}
                                                {this.renderKeyValue('Audio Input RCA Channel Count', device.attributes.audioInputRcaChannelCount)}
                                                {this.renderKeyValue('Audio Input XLR Channel Count', device.attributes.audioInputXlrChannelCount)}
                                                {this.renderKeyValue('Audio Output RCA Channel Count', device.attributes.audioOutputRcaChannelCount)}
                                                {this.renderKeyValue('Audio Output XLR Channel Count', device.attributes.audioOutputXlrChannelCount)}
                                                {this.renderKeyValue('Paired Device Persistent Id', device.attributes.pairedDevicePersistentId, {hex: true})}
                                                {this.renderKeyValue('Video Input Gain Minimum', device.attributes.videoInputGainMinimum)}
                                                {this.renderKeyValue('Video Input Gain Maximum', device.attributes.videoInputGainMaximum)}
                                                {this.renderKeyValue('Video Output Gain Minimum', device.attributes.videoOutputGainMinimum)}
                                                {this.renderKeyValue('Video Output Gain Maximum', device.attributes.videoOutputGainMaximum)}
                                                {this.renderKeyValue('Microphone Input Gain Minimum', device.attributes.microphoneInputGainMinimum)}
                                                {this.renderKeyValue('Microphone Input Gain Maximum', device.attributes.microphoneInputGainMaximum)}
                                                {this.renderKeyValue('Serial Port Device Name', device.attributes.serialPortDeviceName)}
                                                {this.renderKeyValue('Vendor Name', device.attributes.vendorName)}
                                                {this.renderKeyValue('Display Name', device.attributes.displayName)}
                                                {this.renderKeyValue('Model Name', device.attributes.modelName)}
                                                {this.renderKeyValue('Device Handle', device.attributes.deviceHandle)}
                                            </Grid>
                                        </Paper>
                                    ))}
                                </React.Fragment>
                            );
                        }}
                    </AgentQuery>
                </div>
            );
        }
    },
);
