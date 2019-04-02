const HistoryAPIFallback = require('connect-history-api-fallback');
const KoaConnect = require('koa-connect');
const HtmlWebpackPlugin = require('html-webpack-plugin');

const childProcess = require('child_process');
const path = require('path');

const webpack = require('webpack');

function generate() {
    childProcess.execSync('npm run generate', { stdio: 'inherit' });
}

const generator = function() {
    this.hooks.invalid.tap('generator', generate);
    this.hooks.beforeRun.tap('generator', generate);
    this.hooks.afterEnvironment.tap('generator', generate);
};

var config = {
    entry: './src/index.tsx',

    output: {
        filename: '[name].[hash].js',
        path: __dirname + '/dist'
    },

    mode: 'production',

    resolve: {
        extensions: ['.ts', '.tsx', '.js', '.json']
    },

    optimization: {
        minimize: false,
    },

    module: {
        rules: [
            {
                test: /\.tsx?$/,
                loader: 'awesome-typescript-loader',
            },
            {
                test: /\.css$/,
                use: [
                    { loader: 'style-loader' },
                    { loader: 'css-loader' },
                ]
            },
            {
                test: /\.svg$/,
                loader: 'file-loader?publicPath=/',
            }
        ]
    },

    watchOptions: {
        ignored: path.join(__dirname, 'src/__generated__'),
    },

    plugins: [
        generator,
        new HtmlWebpackPlugin({
            filename: 'index.html',
            inject: false,
            template: 'src/index.html',
            templateParameters: (compilation, assets, options) => {
                return {
                    apiHost: process.env.API_HOST,
                    htmlWebpackPlugin: {
                        files: assets,
                        options,
                    },
                };
            },
        }),
        new webpack.ContextReplacementPlugin(/graphql-language-service-interface[\\/]dist$/, new RegExp(`^\\./.*\\.js$`)),
    ],
};

if (process.env.WEBPACK_MODE === 'webpack-serve') {
    config.mode = 'development';
    config.devtool = 'source-map';
    config.module.rules.push({
        enforce: 'pre',
        test: /\.js$/,
        loader: 'source-map-loader',
        exclude: [/node_modules/],
    });
    config.serve = {
        'port': 8082,
        'add': (app, middleware, options) => {
            app.use(KoaConnect(HistoryAPIFallback({})));
        },
    };
}

module.exports = config;
