#!groovy

import hudson.tasks.test.AbstractTestResultAction
import hudson.model.Actionable

// If you ever change something in the methods below, please add a comment.
// Currently, they are in your Jenkinsfile but are intended to go to a shared library

def GetPipelineDockerContainers(String prefix) {
    if (prefix == '') {
        echo "GetPipelineDockerContainers: No prefix passed -> Return"
        return []
    }

    containersStr = sh(script: "docker ps --filter=name=${prefix} -qa", returnStdout: true)
    containers = containersStr.split('\n')

    echo "GetPipelineDockerContainers: Found Containers ${containers}"

    return containers
}

def GetPipelineDockerImages(String prefix) {
    if (prefix == '') {
        echo "GetPipelineDockerImages: No prefix passed -> Return"
        return []
    }

    imagesStr = sh(script: "docker image list --filter=reference=${prefix}* --format ' {{ .Repository }}:{{ .Tag }} '", returnStdout: true)
    images = imagesStr.split('\n')

    echo "GetPipelineDockerImages: Found Images: ${images}"

    return images
}

def GetPipelineDockerVolumes(String prefix) {
    if (prefix == '') {
        echo "GetPipelineDockerVolumes: No prefix passed -> Return"
        return []
    }

    volumesStr = sh(script: "docker volume list --filter=name=${prefix} -q", returnStdout: true)
    volumes = volumesStr.split('\n')

    echo "GetPipelineDockerVolumes: Found Volumes: ${volumes}"

    return volumes
}

def GetPipelineDockerNetworks(String prefix) {
    if (prefix == '') {
        echo "GetPipelineDockerNetworks: No prefix passed -> Return"
        return []
    }

    networksStr = sh(script: "docker network list --filter=name=${prefix} -q", returnStdout: true)
    networks = networksStr.split('\n')

    echo "GetPipelineDockerNetworks: Found Networks: ${networks}"

    return networks
}

def cleanPipelineDockerArtifacts(String prefix) {
    echo "cleaning docker artifacts with prefix: [${prefix}]"

    if (prefix == '') {
        echo "cleanPipelineDockerArtifacts: No prefix passed -> Return"
        return
    }

    containers = GetPipelineDockerContainers(prefix)
    images = GetPipelineDockerImages(prefix)
    volumes = GetPipelineDockerVolumes(prefix)
    networks = GetPipelineDockerNetworks(prefix)

    echo "Will Stop Containers: ${containers}"
    echo "Will Remove Containers: ${containers}"
    echo "Will Remove Images: ${images}"
    echo "Will Remove Volumes: ${volumes}"
    echo "Will Remove Networks: ${networks}"

    containers.each { container ->
        try {
            if (container != '') {
                echo "Stopping container [${container}]"
                sh """
                    docker stop ${container}
                """
            }
        } catch(Exception ex) {
            echo "Exception when Stopping container ${container}: ${ex}"
        }

        try {
            if (container != '') {
                echo "Removing container [${container}]"
                sh """
                    docker rm ${container}
                """
            }
        } catch(Exception ex) {
            echo "Exception when Removing container ${container}: ${ex}"
        }
    }

    images.each { image ->
        try {
            if (image != '') {
                echo "Removing Image [${image}]"
                sh """
                    docker image rm ${image}
                """
            }
        } catch(Exception ex) {
            echo "Exception when Removing Image ${image}: ${ex}"
        }
    }

    volumes.each { volume ->
        try {
            if (volume != '') {
                echo "Removing Volume [${volume}]"
                sh """
                    docker volume rm ${volume}
                """
            }
        } catch(Exception ex) {
            echo "Exception when Removing Volume ${volume}: ${ex}"
        }
    }

    networks.each { network ->
        try {
            if (network != '') {
                echo "Removing Network [${network}]"
                sh """
                    docker network rm ${network}
                """
            }
        } catch(Exception ex) {
            echo "Exception when Removing Network ${network}: ${ex}"
        }
    }
}

pipeline  {
    agent { node { label 'docker_linux' } }

    options {
        disableConcurrentBuilds()
    }

    environment {
        //trigger
        IMAGE_NAME = "${env.BUILD_TAG.toLowerCase().replaceAll("\\s","").replaceAll("%2f","").replaceAll("\\\\","")}"
        PR_NUMBER = "${env.BRANCH_NAME.replace("PR-","")}"
        COMPOSE_PROJECT_NAME = "${IMAGE_NAME}"
        // APP_VERSION = "${env.BRANCH_NAME}-${env.BUILD_NUMBER}"
        // WE FORCE APP_VERSION to be `latest` (for now !)
        APP_VERSION = "latest"
        SLACK_CHANNEL = "#pop-model-status"


    }

    stages {
        stage("Init") {
            steps {
                script {
                    sh '''
                        set +x
                        echo "******************************************************************************************************************************
                                                      Stage Init
                        ******************************************************************************************************************************"
                        set -x
                    '''


                    GIT_COMMIT_USER = sh (
                      script: 'git show -s --pretty=%an',
                      returnStdout: true
                    ).trim()

                    GIT_MESSAGE = sh (
                      script: 'git log -1 --pretty=%B',
                      returnStdout: true
                    ).trim()


                }


                // Create Docker Network //
                // You don't need that
                //wrap([$class: 'AnsiColorBuildWrapper', 'colorMapName': 'xterm']) {
                //    sh '''
                //        set +x
                //        . config/ci.env > /dev/null
                //        set -x
                //        docker network rm ${INTER_STACK_SHARED_NETWORK_NAME} || docker network create --attachable ${INTER_STACK_SHARED_NETWORK_NAME} || true
                //    '''
                //}

                sh """
                    set +x
                    echo "Initialized Variables:
        APP_VERSION=${APP_VERSION}
        COMPOSE_PROJECT_NAME=${COMPOSE_PROJECT_NAME}
        GIT_COMMIT_USER=${GIT_COMMIT_USER}
        GIT_MESSAGE=${GIT_MESSAGE}

        IMAGE_NAME=${IMAGE_NAME}
        PR_NUMBER=${PR_NUMBER}

        SLACK_CHANNEL=${SLACK_CHANNEL}

*************************************************************************************"
                    set -x
                """
            }
        }

        stage('Build images') {
            steps {

                    sh '''
                      set +x
                      echo "******************************************************************************************************************************
                                                      Stage Build Images
******************************************************************************************************************************"
                      set -x
                    '''
                // build the first stage of the docker containers. Tag it so it won't be pruned/deleted
                wrap([$class: 'AnsiColorBuildWrapper', 'colorMapName': 'xterm']) {
                    sh """

                        mkdir -p /tmp/bldg-agg/cargo_home || true
                        mkdir -p /tmp/bldg-agg/rust_target_dir || true
                        mkdir -p /tmp/bldg-agg/rust_target_dir2 || true

                        docker buildx build --target  bldg-agg-python  \
                          --tag ${COMPOSE_PROJECT_NAME}_bldg-agg-python:latest \
                          --file ./docker/bldg-agg-python/Dockerfile .


                    """
                }
            }
            post {
                failure {
                    slackSend channel: "${SLACK_CHANNEL}", color:"danger", message:"""Build Failed - <https://github.com/novelt/bldg-agg/pull/${env.PR_NUMBER}|${env.BRANCH_NAME}> ${env.BUILD_NUMBER} (<${env.BUILD_URL}|Open Jenkins>)
                    Commit by: ${GIT_COMMIT_USER}
                    Message: ${GIT_MESSAGE}
                    """
                }
            }
        }

        stage('Test python gdal build and rust') {

            steps {

                sh '''
                  set +x
                  echo "******************************************************************************************************************************
                                                  Stage Test Python imports
******************************************************************************************************************************"
                  set -x
                '''
                wrap([$class: 'AnsiColorBuildWrapper', 'colorMapName': 'xterm']) {
                    sh '''
                        set -x

                        cd ./docker
                        touch local.env
                        export BLDG_AGG_PYTHON_IMAGE=${COMPOSE_PROJECT_NAME}_bldg-agg-python
                        export DOCKER_CLIENT_TIMEOUT=180
                        export COMPOSE_HTTP_TIMEOUT=180
                        docker-compose -f docker-compose.yml -f docker-compose-jenkins.yml -f down -v --rmi local --remove-orphans
                        docker-compose -f docker-compose.yml -f docker-compose-jenkins.yml up -d db
                        docker-compose -f docker-compose.yml -f docker-compose-jenkins.yml run --rm bldg-agg-python python3.8 -c "from osgeo import gdal; from osgeo import osr; print(gdal.__version__)"

                        docker-compose -f docker-compose.yml -f docker-compose-jenkins.yml run bldg-agg-python /run_tests.sh

                        DOCKER_POP_PYTHON_IMAGE=$(docker ps -a --filter="ancestor=${COMPOSE_PROJECT_NAME}_bldg-agg-python" -q --last 1)
                        docker cp ${DOCKER_POP_PYTHON_IMAGE}:/test_results ".."
                    '''
                }
            }
            post {
                always {
                     junit 'test_results/*_test.xml'
                     cobertura coberturaReportFile: 'test_results/*_cobertura.xml'
                }
                failure {
                    slackSend channel: "${SLACK_CHANNEL}", color:"danger", message:"""Tests Failed - <https://github.com/novelt/bldg-agg/pull/${env.PR_NUMBER}|${env.BRANCH_NAME}> ${env.BUILD_NUMBER} (<${env.BUILD_URL}|Open Jenkins>)
                    Commit by: ${GIT_COMMIT_USER}
                    Message: ${GIT_MESSAGE}
                    """
                }
            }
        }
    }
    post {
        cleanup {

            echo """
                ******************************************************************************************************************************
                                              Post build - Cleanup
                ******************************************************************************************************************************
            """.stripIndent()

            cleanPipelineDockerArtifacts(env.COMPOSE_PROJECT_NAME)

            sh """
                rm -rf ./test_results
            """
        }
    }
}
