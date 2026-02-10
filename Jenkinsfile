pipeline {
    agent any

    environment {
        PATH = "/root/.cargo/bin:$PATH" 
    }

    stages {
        stage('Checkout') {
            steps {
                checkout scm
            }
        }
        

        stage('Build WASM') {
            steps {
                sh 'cargo build --target wasm32-unknown-unknown --release'
                
                sh '''
                    wasm-bindgen --target web \
                                 --no-typescript \
                                 --out-dir www/pkg \
                                 target/wasm32-unknown-unknown/release/gpudemo.wasm
                '''
            }
        }

        stage('Archive Artifacts') {
            steps {
                archiveArtifacts artifacts: """
                    target/wasm32-unknown-unknown/release/gpudemo.wasm, 
                    www/pkg/**/*
                """, 
                fingerprint: true,
                allowEmptyArchive: false
            }
        }
    }
}
